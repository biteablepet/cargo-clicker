use std::{
    borrow::Cow,
    env,
    error::Error,
    ffi::OsStr,
    fs::{self, File},
    io::{BufReader, Cursor, Read, Seek},
    path::Path,
    process::{self, Command},
    str::FromStr,
};

use rand::prelude::*;
use rodio::{Decoder, OutputStream, Sink, Source};

// if JUST_CLICK_VAR is set, we will just play sound and exit, detaching
// the execution of the build from the need to wait for sounds to finish
const JUST_CLICK_VAR: &str = "__CARGO_CLICKER_PLAYING_SOUND";

// if CARGO_REPLACEMENT_VAR is set, its value is the next binary invoked
// instead of cargo
// if SILENCE_VAR is set, no sound will ever be played and the result of
// the invocation will always be directly emitted
// if RECURSION_PREVENTION_VAR is set, no sound will be played under the
// assumption that we're being invoked recursively and this could sound,
// well, really really bad
const CARGO_REPLACEMENT_VAR: &str = "CARGO_CLICKER_ACTUAL";
const SILENCE_VAR: &str = "CARGO_CLICKER_SILENCE";
const RECURSION_PREVENTION_VAR: &str = "__CARGO_CLICKER_INSIDE_CARGO_CLICKER";

const RESPONSE_DIR_VAR: &str = "CARGO_CLICKER_RESPONSES";

const BAKED_POSITIVE_RESPONSES: &[&[u8]] = &[
    include_bytes!("builtin-sounds/positive0.mp3"),
    include_bytes!("builtin-sounds/positive1.mp3"),
    include_bytes!("builtin-sounds/positive2.mp3"),
    include_bytes!("builtin-sounds/positive3.mp3"),
];
const BAKED_NEGATIVE_RESPONSES: &[&[u8]] = &[];

enum Response {
    Positive,
    Negative,
}

impl ToString for Response {
    fn to_string(&self) -> String {
        match self {
            Response::Positive => "Positive".to_owned(),
            Response::Negative => "Negative".to_owned(),
        }
    }
}

impl FromStr for Response {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Positive" => Ok(Response::Positive),
            "Negative" => Ok(Response::Negative),
            _ => Err("unknown Response"),
        }
    }
}

fn main() {
    let code = real_main().unwrap_or_else(|e| {
        let this_bin = env::current_exe()
            .map(Cow::from)
            .unwrap_or_else(|_| Path::new("cargo-clicker").into());
        let this_bin = this_bin.display();
        eprintln!("{this_bin} - error: {e:?}");
        1
    });
    process::exit(code)
}

fn real_main() -> Result<i32, Box<dyn Error>> {
    if let Ok(kind) = env::var(JUST_CLICK_VAR) {
        play_sound_and_block(kind.parse()?)?;
        return Ok(0);
    }

    let mut args = env::args().peekable();

    // we can execute anything rather than cargo itself, like if we want
    // to run `cargo-mommy` next for an especially good puppy
    let cargo = env::var(CARGO_REPLACEMENT_VAR)
        .or_else(|_| env::var("CARGO"))
        .map(Cow::from)
        .unwrap_or_else(|_| "cargo".into());

    // if we're invoked recursively we don't actually want to play audio
    // because while playing fifty overlapping sounds could be silly, we
    // think that sounds exhausting for a puppy

    let silenced = env::var(SILENCE_VAR).is_ok() || env::var(RECURSION_PREVENTION_VAR).is_ok();

    // we run as `cargo-clicker [args...]` or `cargo clicker [args...]`.
    // we need to strip off any prefixed repetitions of clicker (to skip
    // the need to recursively execute ourself for very excited puppies)
    let _ = args.next();
    while let Some(next_argument) = args.peek() {
        if next_argument != "clicker" {
            break;
        }

        // Skip this argument
        let _ = args.next();
    }

    let mut cmd = Command::new(&*cargo);
    cmd.args(args).env(RECURSION_PREVENTION_VAR, "1");
    let status = cmd
        .status()
        .map_err(|err| format!("couldn't find a `{cargo}`: {err}"))?;
    let code = status.code().unwrap_or(1);
    if !silenced
        && !matches!(
            cmd.get_args()
                .filter_map(OsStr::to_str)
                .try_for_each(|arg| match arg.as_bytes() {
                    b"--" => Err(false),
                    b"--quiet" => Err(true),
                    [b'-', b'-', ..] => Ok(()),
                    [b'-', args @ ..] if args.contains(&b'q') => Err(true),
                    _ => Ok(()),
                }),
            Err(true)
        )
    {
        respond(if status.success() {
            Response::Positive
        } else {
            Response::Negative
        })?;
    }

    Ok(code)
}

fn respond(kind: Response) -> Result<(), Box<dyn Error>> {
    let mut cmd = Command::new(env::current_exe()?);
    cmd.env(JUST_CLICK_VAR, kind.to_string());
    cmd.spawn()?;

    Ok(())
}

trait DynamicSource: Seek + Read + Send + Sync {}
impl<T: Seek + Read + Send + Sync> DynamicSource for T {}

fn get_sound(
    mut rng: &mut ThreadRng,
    kind: Response,
) -> Result<Option<Box<dyn DynamicSource>>, Box<dyn Error>> {
    let sound = match env::var(RESPONSE_DIR_VAR) {
        Ok(response_dir) => {
            let dir = fs::read_dir(Path::new(&response_dir).join(kind.to_string()))?;
            let Some(entry) = dir
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .choose(&mut rng)
            else {
                return Ok(None);
            };

            Some(Box::new(BufReader::new(File::open(entry.path())?)) as Box<dyn DynamicSource>)
        }
        Err(_) => {
            let responses = match kind {
                Response::Positive => BAKED_POSITIVE_RESPONSES.choose(&mut rng),
                Response::Negative => BAKED_NEGATIVE_RESPONSES.choose(&mut rng),
            };
            let Some(bytes) = responses.into_iter().choose(rng) else {
                return Ok(None);
            };

            Some(Box::new(Cursor::new(bytes)) as Box<dyn DynamicSource>)
        }
    };

    Ok(sound)
}

fn play_sound_and_block(kind: Response) -> Result<(), Box<dyn Error>> {
    let mut rng = rand::rng();

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let Some(source) = get_sound(&mut rng, kind)? else {
        // there's nothing to play for an empty set of responses
        return Ok(());
    };

    let source = Decoder::new(source)?.speed(rng.random_range(0.95..1.05));

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}
