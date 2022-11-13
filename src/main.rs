#[macro_use]
mod macros;
mod patterns;
mod color_value;

use crate::patterns::{Patterns, RegexPatterns};
use clap::Parser;
use color_value::ColorValue;
use once_cell::sync::Lazy;
use parking_lot::{Mutex, RwLock};
use rand::thread_rng;
use regex::Regex;
use secp256k1::Secp256k1;
use sha3::Digest;
use std::{fmt::Write, sync::Arc, thread, time::Duration};
use termcolor::{Buffer, BufferWriter, Color};
use typenum::U40;

type _AddressLengthType = U40;

const ADDRESS_LENGTH: usize = 40;
const ADDRESS_BYTES: usize = ADDRESS_LENGTH / 2;
const KECCAK_OUTPUT_BYTES: usize = 32;
const ADDRESS_BYTE_INDEX: usize = KECCAK_OUTPUT_BYTES - ADDRESS_BYTES;

static _ADDRESS_PATTERN: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[0-9a-f]{1,40}$").unwrap());

#[derive(Debug, Clone)]
struct BruteforceResult {
    address: String,
    private_key: String,
}

fn to_hex_string(slice: &[u8], expected_string_size: usize) -> String {
    let mut result = String::with_capacity(expected_string_size);

    for &byte in slice {
        write!(&mut result, "{:02x}", byte).expect("Unable to format the public key.");
    }

    result
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[clap(
        short = 'q',
        long = "quiet",
        help = "Output only the results",
        long_help = "Output only the resulting address and private key separated by a space."
    )]
    quiet: bool,
    #[clap(
        short = 'c',
        long = "color",
        help = "Changes the color formatting strategy",
        long_help = "Changes the color formatting strategy in the following way:
        always      -- Try very hard to emit colors. This includes
                       emitting ANSI colors on Windows if the console
                       API is unavailable.
        always_ansi -- like always, except it never tries to use
                       anything other than emitting ANSI color codes.
        auto        -- Try to use colors, but don't force the issue.
                       If the console isn't available on Windows, or
                       if TERM=dumb, for example, then don't use colors.
        never       -- Never emit colors."
    )]
    #[arg(value_enum)]
    color: Option<ColorValue>,
    #[clap(
        short = 's',
        long = "stream",
        help = "Keep outputting results",
        long_help = "Instead of outputting a single result, keep outputting until terminated."
    )]
    stream: bool,
    #[clap(
        value_name = "PATTERN",
        help = "The pattern to match the address against",
        long_help = "The regex pattern to match the address against.
        If no patterns are provided, they are read from the stdin (standard input),
        where each pattern is on a separate line.
        Addresses are output if the beginning matches one of these patterns."
    )]
    pattern: Vec<String>,
}

fn main() {
    let args = Args::parse();
    let color_choice = args.color.unwrap_or_default().into();
    let buffer_writer = Arc::new(Mutex::new(BufferWriter::stdout(color_choice)));

    let patterns = Arc::new(RegexPatterns::new(&args.pattern[..]));
    main_pattern_type_selected(&args, buffer_writer, patterns);
}

fn main_pattern_type_selected<P: Patterns + 'static>(
    args: &Args,
    buffer_writer: Arc<Mutex<BufferWriter>>,
    patterns: Arc<P>,
) {
    if patterns.len() == 0 {
        let mut stdout = buffer_writer.lock().buffer();
        cprintln!(
            false,
            stdout,
            Color::Red,
            "Please, provide at least one valid pattern."
        );
        buffer_writer
            .lock()
            .print(&stdout)
            .expect("Could not write to stdout.");
        std::process::exit(1);
    }

    {
        let mut stdout = buffer_writer.lock().buffer();
        cprintln!(args.quiet,
                  stdout,
                  Color::White,
                  "---------------------------------------------------------------------------------------");

        if patterns.len() <= 1 {
            cprint!(
                args.quiet,
                stdout,
                Color::White,
                "Looking for an address matching "
            );
        } else {
            cprint!(
                args.quiet,
                stdout,
                Color::White,
                "Looking for an address matching any of "
            );
        }

        cprint!(args.quiet, stdout, Color::Cyan, "{}", patterns.len());

        if patterns.len() <= 1 {
            cprint!(args.quiet, stdout, Color::White, " pattern");
        } else {
            cprint!(args.quiet, stdout, Color::White, " patterns");
        }

        cprintln!(args.quiet, stdout, Color::White, "");
        cprintln!(args.quiet,
                  stdout,
                  Color::White,
                  "---------------------------------------------------------------------------------------");
        buffer_writer
            .lock()
            .print(&stdout)
            .expect("Could not write to stdout.");
    }

    let thread_count = num_cpus::get();

    loop {
        let mut threads = Vec::with_capacity(thread_count);
        let result: Arc<RwLock<Option<BruteforceResult>>> = Arc::new(RwLock::new(None));
        let iterations_this_second: Arc<RwLock<u32>> = Arc::new(RwLock::new(0));
        let alg = Arc::new(Secp256k1::new());
        let working_threads = Arc::new(RwLock::new(thread_count));

        for _ in 0..thread_count {
            let working_threads = working_threads.clone();
            let patterns = patterns.clone();
            let result = result.clone();
            let alg = alg.clone();
            let iterations_this_second = iterations_this_second.clone();

            threads.push(thread::spawn(move || {
                'dance: loop {
                    if result.read().is_some() {
                        break 'dance;
                    }

                    let mut rng = thread_rng();
                    let (private_key, public_key) = alg.generate_keypair(&mut rng);
                    let public_key_array = public_key.serialize();
                    let mut keccak = sha3::Keccak256::new();
                    keccak.update(public_key_array);
                    let keccak_result = keccak.finalize();
                    // let keccak = tiny_keccak::keccak256(public_key_array);
                    let address = to_hex_string(&keccak_result[ADDRESS_BYTE_INDEX..], 40); // get rid of the constant 0x04 byte

                    if patterns.contains(&address) {
                        *result.write() = Some(BruteforceResult {
                            address,
                            private_key: to_hex_string(&private_key[..], 64),
                        });
                        break 'dance;
                    }

                    *iterations_this_second.write() += 1;
                }

                *working_threads.write() -= 1;
            }));
        }

        // Note:
        // Buffers are intended for correct concurrency.
        let sync_buffer: Arc<RwLock<Option<Buffer>>> = Arc::new(RwLock::new(None));

        {
            let buffer_writer = buffer_writer.clone();
            let sync_buffer = sync_buffer.clone();
            let result = result.clone();
            let quiet = args.quiet;
            thread::spawn(move || 'dance: loop {
                thread::sleep(Duration::from_secs(1));

                if result.read().is_some() {
                    break 'dance;
                }

                let mut buffer = buffer_writer.lock().buffer();
                cprint!(
                    quiet,
                    buffer,
                    Color::Cyan,
                    "{}",
                    *iterations_this_second.read()
                );
                cprintln!(quiet, buffer, Color::White, " addresses / second");
                *iterations_this_second.write() = 0;
                *sync_buffer.write() = Some(buffer);
            });
        }

        'dance: loop {
            if *working_threads.read() == 0 {
                break 'dance;
            }

            if let Some(ref buffer) = *sync_buffer.read() {
                buffer_writer
                    .lock()
                    .print(buffer)
                    .expect("Could not write to stdout.");
            }

            *sync_buffer.write() = None;

            thread::sleep(Duration::from_millis(10));
        }

        for thread in threads {
            thread.join().unwrap();
        }

        let result = result.read();
        let result = result.as_ref().unwrap();

        {
            let mut stdout = buffer_writer.lock().buffer();
            cprintln!(args.quiet,
                      stdout,
                      Color::White,
                      "---------------------------------------------------------------------------------------");
            cprint!(args.quiet, stdout, Color::White, "Found address: ");
            cprintln!(args.quiet, stdout, Color::Yellow, "0x{}", result.address);
            cprint!(args.quiet, stdout, Color::White, "Generated private key: ");
            cprintln!(args.quiet, stdout, Color::Red, "{}", result.private_key);
            cprintln!(
                args.quiet,
                stdout,
                Color::White,
                "Import this private key into an ethereum wallet in order to use the address."
            );
            cprintln!(args.quiet,
                      stdout,
                      Color::Green,
                      "Buy me a cup of coffee; my ethereum address: 0xc0ffee3bd37d408910ecab316a07269fc49a20ee");
            cprintln!(args.quiet,
                      stdout,
                      Color::White,
                      "---------------------------------------------------------------------------------------");
            buffer_writer
                .lock()
                .print(&stdout)
                .expect("Could not write to stdout.");
        }

        if args.quiet {
            println!("0x{} {}", result.address, result.private_key);
        }

        if !args.stream {
            break;
        }
    }
}
