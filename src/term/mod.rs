use crate::mach::{Event, Listing, Runtime};
use crate::{error, lang::Error};
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event as CtEvent, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode};
use nu_ansi_term::Style;
use rustyline::completion::Pair;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::line_buffer::LineBuffer;
use rustyline::{Changeset, Cmd, Editor, KeyEvent, Modifiers};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter, Validator};
use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

type Ed = Editor<RsHelper, DefaultHistory>;

pub fn main() {
    if std::env::args().count() > 2 {
        println!("Usage: rsbasic [FILENAME]");
        return;
    }
    let mut args = std::env::args();
    let _executable = args.next();
    let filename = match args.next() {
        Some(f) => f,
        _ => "".into(),
    };
    let interrupted = Arc::new(AtomicBool::new(false));
    let int_moved = interrupted.clone();
    ctrlc::set_handler(move || {
        int_moved.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");
    if let Err(error) = main_loop(interrupted, filename) {
        eprintln!("{}", error);
    }
}

fn main_loop(
    interrupted: Arc<AtomicBool>,
    filename: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut runtime = Runtime::default();
    let mut command: Ed = Editor::new()?;
    let mut input_full: Ed = Editor::new()?;
    let mut input_caps: Ed = Editor::new()?;
    CapsFunction::install(&mut input_caps);
    // Partial output line (text printed since the last newline). Folded into the
    // next readline prompt so a preceding `PRINT "x";` survives rustyline's
    // redraw-at-column-0 (e.g. `?"Why";:INPUT Y` shows `Why? `).
    let mut pending = String::new();

    if !filename.is_empty() {
        match load(&filename, true, false) {
            Ok(listing) => {
                if listing.is_empty() {
                    return Ok(());
                }
                runtime.set_prompt("");
                runtime.set_listing(listing, true);
            }
            Err(error) => {
                println!("{}", Style::new().bold().paint(error.to_string()));
                return Ok(());
            }
        }
    }

    loop {
        if interrupted.load(Ordering::SeqCst) {
            runtime.interrupt();
            interrupted.store(false, Ordering::SeqCst);
        };
        match runtime.execute(5000) {
            Event::Stopped => {
                if !filename.is_empty() {
                    return Ok(());
                }
                command.set_helper(Some(RsHelper::with_completer(LineCompleter::new(
                    runtime.get_listing(),
                ))));
                // Take pending before readline so it is cleared on every path below
                // (including the Ctrl-C `continue`).
                let cmd_prompt = std::mem::take(&mut pending);
                let string = match command.readline(&cmd_prompt) {
                    Ok(string) => string,
                    // Ctrl-C: cancel the line and re-prompt (runtime is still Stopped).
                    Err(ReadlineError::Interrupted) => {
                        command.set_helper(None);
                        continue;
                    }
                    // Ctrl-D (EOF) exits the app.
                    Err(ReadlineError::Eof) => break,
                    Err(e) => return Err(e.into()),
                };
                command.set_helper(None);
                if runtime.enter(&string) {
                    command.add_history_entry(string)?;
                }
            }
            Event::Input(prompt, caps) => {
                let input = if caps { &mut input_caps } else { &mut input_full };
                // Fold any preceding partial output line into the prompt so rustyline
                // redraws it at column 0 (e.g. `?"Why";:INPUT Y` -> `Why? `).
                let full_prompt = format!("{}{}", pending, prompt);
                let result = input.readline(&full_prompt);
                pending.clear();
                match result {
                    Ok(string) => {
                        if runtime.enter(&string) {
                            input.add_history_entry(string)?;
                        }
                    }
                    Err(ReadlineError::Interrupted) => {
                        runtime.interrupt();
                    }
                    Err(ReadlineError::Eof) => break,
                    Err(e) => return Err(e.into()),
                };
            }
            Event::Errors(errors) => {
                for error in errors.iter() {
                    println!("{}", Style::new().bold().paint(error.to_string()));
                }
                pending.clear();
            }
            Event::Running => {}
            Event::Print(s) => {
                print!("{}", s);
                std::io::stdout().flush().ok();
                // Track the current partial line (text since the last newline).
                match s.rfind('\n') {
                    Some(i) => pending = s[i + 1..].to_string(),
                    None => pending.push_str(&s),
                }
            }
            Event::List((s, columns)) => {
                println!("{}", decorate_list(&s, &columns));
                pending.clear();
            }
            Event::Load(s) => match load(&s, false, false) {
                Ok(listing) => runtime.set_listing(listing, false),
                Err(error) => {
                    println!("{}", Style::new().bold().paint(error.to_string()));
                    pending.clear();
                }
            },
            Event::Run(s) => match load(&s, false, false) {
                Ok(listing) => runtime.set_listing(listing, true),
                Err(error) => {
                    println!("{}", Style::new().bold().paint(error.to_string()));
                    pending.clear();
                }
            },
            Event::Save(s) => match save(&runtime.get_listing(), &s) {
                Ok(_) => {}
                Err(error) => {
                    println!("{}", Style::new().bold().paint(error.to_string()));
                    pending.clear();
                }
            },
            Event::Cls => {
                execute!(std::io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
                pending.clear();
            }
            Event::Inkey => {
                let s = read_inkey()?;
                runtime.enter(&s);
            }
        }
    }
    Ok(())
}

fn read_inkey() -> std::io::Result<std::rc::Rc<str>> {
    enable_raw_mode()?;
    let result = (|| -> std::io::Result<std::rc::Rc<str>> {
        loop {
            if event::poll(std::time::Duration::from_millis(1))? {
                if let CtEvent::Key(key) = event::read()?
                    && (key.kind == KeyEventKind::Press || key.kind == KeyEventKind::Repeat)
                {
                    return Ok(map_key(key.code, key.modifiers));
                }
            } else {
                return Ok("".into());
            }
        }
    })();
    disable_raw_mode()?;
    result
}

fn map_key(code: KeyCode, mods: KeyModifiers) -> std::rc::Rc<str> {
    match code {
        KeyCode::Backspace => "\x08".into(),
        KeyCode::Enter => "\x0D".into(),
        KeyCode::Esc => "\x1B".into(),
        KeyCode::Tab => "\x09".into(),
        KeyCode::Up => "\x00H".into(),
        KeyCode::Down => "\x00P".into(),
        KeyCode::Left => "\x00K".into(),
        KeyCode::Right => "\x00M".into(),
        KeyCode::Delete => "\x00S".into(),
        KeyCode::Insert => "\x00R".into(),
        KeyCode::Home => "\x00G".into(),
        KeyCode::End => "\x00O".into(),
        KeyCode::PageUp => "\x00I".into(),
        KeyCode::PageDown => "\x00Q".into(),
        KeyCode::Char(c) if mods.contains(KeyModifiers::CONTROL) => {
            match std::char::from_u32(c as u32 - 60) {
                Some(c) => c.to_string().into(),
                None => "".into(),
            }
        }
        KeyCode::Char(c) => c.to_string().into(),
        _ => "".into(),
    }
}

struct CapsFunction;

impl CapsFunction {
    fn install(ed: &mut Ed) {
        for ch in b'a'..=b'z' {
            ed.bind_sequence(
                KeyEvent::new(ch as char, Modifiers::NONE),
                Cmd::Insert(1, (ch as char).to_ascii_uppercase().to_string()),
            );
        }
    }
}

#[derive(Helper, Completer, Hinter, Highlighter, Validator)]
struct RsHelper {
    #[rustyline(Completer)]
    completer: LineCompleter,
}

impl RsHelper {
    fn with_completer(completer: LineCompleter) -> RsHelper {
        RsHelper { completer }
    }
}

struct LineCompleter {
    runtime: Listing,
}

impl LineCompleter {
    fn new(runtime: Listing) -> LineCompleter {
        LineCompleter { runtime }
    }
}

impl rustyline::completion::Completer for LineCompleter {
    type Candidate = Pair;
    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        if let Ok(num) = line.trim().parse::<usize>()
            && let Some((s, _)) = self.runtime.line(num)
        {
            return Ok((
                0,
                vec![Pair {
                    display: s.clone(),
                    replacement: s,
                }],
            ));
        }
        Ok((0, vec![]))
    }

    // The default `update` replaces only `start..cursor`, so with the cursor moved
    // off the end the typed line number survives. Replace the whole buffer instead
    // so recall works regardless of cursor position.
    fn update(&self, line: &mut LineBuffer, _start: usize, elected: &str, cl: &mut Changeset) {
        let end = line.len();
        line.replace(0..end, elected, cl);
    }
}

fn decorate_list(ins: &str, columns: &[std::ops::Range<usize>]) -> String {
    let mut under_on = false;
    let mut out = String::new();
    let style = Style::new().underline();
    let prefix = format!("{}", style.prefix());
    let suffix = format!("{}", style.suffix());
    let mut index = 0;
    for char in ins.chars() {
        let do_under = columns.iter().any(|c| c.contains(&index));
        if under_on {
            if !do_under {
                out.push_str(&suffix);
            }
        } else if do_under {
            out.push_str(&prefix);
        }
        under_on = do_under;
        out.push(char);
        index += 1;
    }
    if columns.iter().any(|c| c.start == index) {
        under_on = true;
        out.push_str(&prefix);
        out.push(' ');
    }
    if under_on {
        out.push_str(&suffix);
    }
    out
}

fn save(listing: &Listing, filename: &str) -> Result<(), Error> {
    if listing.is_empty() {
        return Err(error!(InternalError; "NOTHING TO SAVE"));
    }
    let mut file = match fs::File::create(filename) {
        Ok(file) => file,
        Err(error) => return Err(error!(InternalError;  error.to_string().as_str())),
    };
    for line in listing.lines() {
        if let Err(error) = writeln!(file, "{}", line) {
            return Err(error!(InternalError; error.to_string().as_str()));
        }
    }
    Ok(())
}

fn parse_filename(filename: &str, index: usize) -> Result<String, Error> {
    let filename = filename.trim();
    if filename.len() < 3 || !filename.starts_with('"') || !filename.ends_with('"') {
        return Err(error!(BadFileName; &format!(
            "In line {} of the patch file.",
            index + 1
        )));
    }
    let filename = filename[1..filename.len() - 1].to_string();
    match fs::metadata(&filename) {
        Ok(_metadata) => {
            println!("Saving to {}", filename);
            Err(error!(FileAlreadyExists; &format!(
                "In line {} of the patch file.", index+1
            )))
        }
        Err(e) => {
            if let ErrorKind::NotFound = e.kind() {
                Ok(filename)
            } else {
                Err(error!(InternalError; &e.to_string()))
            }
        }
    }
}

fn load(filename: &str, allow_patch: bool, ignore_errors: bool) -> Result<Listing, Error> {
    if filename.starts_with("http://")
        || filename.starts_with("https://")
        || filename.starts_with("//")
    {
        let filename = if let Some(filename) = filename.strip_prefix("//") {
            let mut url =
                "https://raw.githubusercontent.com/rumbledethumps/rsbasic/master/patch/"
                    .to_string();
            url.push_str(filename);
            url
        } else {
            filename.to_string()
        };
        let mut reader = match ureq::get(filename.as_str()).call() {
            Ok(resp) => BufReader::new(resp.into_body().into_reader()),
            Err(ureq::Error::StatusCode(code)) => {
                return Err(error!(FileNotFound; &format!("{}", code)));
            }
            Err(e) => return Err(error!(InternalError; e.to_string().as_str())),
        };
        load2(&mut reader, allow_patch, ignore_errors)
    } else {
        let mut reader = match fs::File::open(filename) {
            Ok(file) => BufReader::new(file),
            Err(error) => {
                let msg = error.to_string();
                match error.kind() {
                    ErrorKind::NotFound => return Err(error!(FileNotFound; msg.as_str())),
                    _ => return Err(error!(InternalError; msg.as_str())),
                }
            }
        };
        load2(&mut reader, allow_patch, ignore_errors)
    }
}

fn load2(
    reader: &mut dyn std::io::BufRead,
    allow_patch: bool,
    ignore_errors: bool,
) -> Result<Listing, Error> {
    let mut first_listing = Listing::default();
    let mut listing = Listing::default();
    let mut patching = false;
    let mut filename = String::default();
    for (index, line) in reader.lines().enumerate() {
        match line {
            Err(error) => return Err(error!(InternalError; error.to_string().as_str())),
            Ok(line) => {
                if allow_patch && index == 0 && (line.starts_with('"') || line.starts_with('\'')) {
                    patching = true;
                    println!("Patch mode.\n");
                }
                if patching && line.starts_with('\'') {
                    println!("{}", line[1..].trim());
                    continue;
                }
                if patching && line.starts_with('"') {
                    let mut parts: Vec<&str> = line.split_ascii_whitespace().collect();
                    if parts.len() == 1 {
                        filename = parse_filename(parts.pop().unwrap(), index)?;
                    } else if parts.len() == 3 {
                        if !filename.is_empty() {
                            println!("Saving to {}", filename);
                            save(&listing, &filename)?;
                            println!();
                        }
                        if first_listing.is_empty() {
                            std::mem::swap(&mut listing, &mut first_listing)
                        }
                        let url = parts.pop().unwrap();
                        let crc = parts.pop().unwrap();
                        filename = parse_filename(parts.pop().unwrap(), index)?;
                        println!("Retrieving from {}", url);
                        listing = load(url, false, true)?;
                        let crc = match u32::from_str_radix(crc, 16) {
                            Ok(crc) => crc,
                            Err(_) => {
                                return Err(error!(SyntaxError; &format!(
                                    "Unable to parse crc info in line {} of the patch file.",
                                    index + 1
                                )));
                            }
                        };
                        const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);
                        let mut digest = CRC.digest();
                        for line in listing.lines() {
                            digest.update(line.to_string().as_bytes());
                        }
                        let digest = digest.finalize();
                        if digest != crc {
                            return Err(error!(SyntaxError; &format!(
                                "Expected CRC {:08X} got {:08X} in line {} of the patch file.",
                                crc, digest, index + 1
                            )));
                        }
                    } else {
                        return Err(error!(SyntaxError; &format!(
                            "Unable to parse info in line {} of the patch file.",
                            index + 1
                        )));
                    }
                    continue;
                }
                if let Err(error) = listing.load_str(&line)
                    && !ignore_errors
                {
                    return Err(error.message(&format!("In line {} of the file.", index + 1)));
                }
            }
        }
    }
    if patching {
        println!("Saving to {}", filename);
        save(&listing, &filename)?;
        println!();
        if !first_listing.is_empty() {
            Ok(first_listing)
        } else {
            Ok(listing)
        }
    } else {
        Ok(listing)
    }
}
