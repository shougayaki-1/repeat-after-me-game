//! Repeat After Me Game exhaustive explorer. No external crates required.
//! Tokens: RAM=0, KON=1, nRAM=2, nKON=3; nRAM transforms t to t^1.
use std::collections::BTreeMap;
use std::env;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::thread;

const NAMES: [&str; 4] = ["RAM", "KON", "nRAM", "nKON"];
const STATUS_NAMES: [&str; 4] = ["done", "cycle", "token_limit", "round_limit"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Status { Done, Cycle, TokenLimit, RoundLimit }
impl Status { fn index(self) -> usize { self as usize } fn as_str(self) -> &'static str { STATUS_NAMES[self.index()] } }

#[derive(Clone, Copy)]
struct Limits { max_rounds: usize, max_tokens: usize }

#[derive(Debug)]
struct Outcome {
    status: Status,
    rounds: usize,
    max_tokens_seen: usize,
    final_tokens: Vec<u8>,
    cycle_start: Option<usize>,
    cycle_period: Option<usize>,
    commands_executed: u64,
    attempted_tokens: Option<usize>,
}

fn is_command(token: u8) -> bool { token == 0 || token == 2 }
fn next_size(input: &[u8]) -> (usize, usize) {
    let mut output_len = 0usize;
    let mut commands = 0usize;
    for (i, &token) in input.iter().enumerate() {
        if is_command(token) {
            commands += 1;
            output_len = output_len.saturating_add(input.len() - i - 1);
        }
    }
    (output_len, commands)
}

fn step(input: &[u8], capacity: usize) -> Vec<u8> {
    let mut next = Vec::with_capacity(capacity);
    for (i, &token) in input.iter().enumerate() {
        match token {
            0 => next.extend_from_slice(&input[i + 1..]),
            2 => next.extend(input[i + 1..].iter().map(|&x| x ^ 1)),
            _ => {}
        }
    }
    debug_assert_eq!(next.len(), capacity);
    next
}

fn simulate(input: &[u8], limits: Limits) -> Outcome {
    let mut current = input.to_vec();
    let mut seen: Vec<Vec<u8>> = vec![current.clone()];
    let mut max_tokens_seen = current.len();
    let mut commands_executed = 0u64;
    for round in 1..=limits.max_rounds {
        let (size, commands) = next_size(&current);
        if commands == 0 {
            return Outcome { status: Status::Done, rounds: round - 1, max_tokens_seen,
                final_tokens: current, cycle_start: None, cycle_period: None,
                commands_executed, attempted_tokens: None };
        }
        if size > limits.max_tokens {
            return Outcome { status: Status::TokenLimit, rounds: round - 1, max_tokens_seen,
                final_tokens: current, cycle_start: None, cycle_period: None,
                commands_executed, attempted_tokens: Some(size) };
        }
        let next = step(&current, size);
        commands_executed += commands as u64;
        max_tokens_seen = max_tokens_seen.max(next.len());
        if !next.iter().any(|&t| is_command(t)) {
            return Outcome { status: Status::Done, rounds: round, max_tokens_seen,
                final_tokens: next, cycle_start: None, cycle_period: None,
                commands_executed, attempted_tokens: None };
        }
        if let Some(start) = seen.iter().position(|s| *s == next) {
            return Outcome { status: Status::Cycle, rounds: round, max_tokens_seen,
                final_tokens: next, cycle_start: Some(start), cycle_period: Some(round - start),
                commands_executed, attempted_tokens: None };
        }
        if round == limits.max_rounds {
            return Outcome { status: Status::RoundLimit, rounds: round, max_tokens_seen,
                final_tokens: next, cycle_start: None, cycle_period: None,
                commands_executed, attempted_tokens: None };
        }
        seen.push(next.clone());
        current = next;
    }
    unreachable!("max_rounds must be greater than zero")
}

fn format_tokens(tokens: &[u8]) -> String {
    tokens.iter().map(|&t| NAMES[t as usize]).collect::<Vec<_>>().join(" ")
}
fn preview(tokens: &[u8]) -> String {
    if tokens.len() <= 32 { return format_tokens(tokens); }
    format!("{} ... [{} omitted] ... {}", format_tokens(&tokens[..16]),
        tokens.len() - 32, format_tokens(&tokens[tokens.len()-16..]))
}
fn parse_tokens(s: &str) -> Result<Vec<u8>, String> {
    let tokens: Result<Vec<_>, _> = s.split_whitespace().map(|part| {
        NAMES.iter().position(|name| name.eq_ignore_ascii_case(part))
            .map(|i| i as u8).ok_or_else(|| format!("unknown token: {part}"))
    }).collect();
    let tokens = tokens?;
    if tokens.is_empty() { return Err("input must not be empty".into()); }
    Ok(tokens)
}
fn decode(mut code: u64, length: usize) -> Vec<u8> {
    let mut tokens = vec![0; length];
    for i in (0..length).rev() { tokens[i] = (code & 3) as u8; code >>= 2; }
    tokens
}
fn splitmix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

#[derive(Clone)]
struct Example { input: Vec<u8>, outcome: OutcomeSummary }
#[derive(Clone)]
struct OutcomeSummary {
    status: Status, rounds: usize, max_tokens_seen: usize, final_tokens: Vec<u8>,
    cycle_period: Option<usize>, attempted_tokens: Option<usize>,
}
impl From<Outcome> for OutcomeSummary {
    fn from(o: Outcome) -> Self { Self { status:o.status, rounds:o.rounds,
        max_tokens_seen:o.max_tokens_seen, final_tokens:o.final_tokens,
        cycle_period:o.cycle_period, attempted_tokens:o.attempted_tokens } }
}
fn sample_score(o: &OutcomeSummary) -> (usize, usize, usize) {
    match o.status {
        Status::Done => (o.rounds, o.max_tokens_seen, o.final_tokens.len()),
        Status::Cycle => (o.cycle_period.unwrap_or(0), o.rounds, o.max_tokens_seen),
        Status::TokenLimit => (o.attempted_tokens.unwrap_or(0), o.rounds, o.max_tokens_seen),
        Status::RoundLimit => (o.max_tokens_seen, o.rounds, o.final_tokens.len()),
    }
}
#[derive(Default)]
struct Stats {
    cases: u64,
    statuses: [u64; 4],
    done_empty: u64,
    total_rounds: u128,
    total_commands: u128,
    max_rounds_seen: usize,
    max_tokens_seen: usize,
    histogram: BTreeMap<(Status, usize), u64>,
    examples: [Option<Example>; 4],
    largest: Option<Example>,
}
impl Stats {
    fn add(&mut self, input: Vec<u8>, o: Outcome) {
        self.cases += 1;
        self.statuses[o.status.index()] += 1;
        self.done_empty += u64::from(o.status == Status::Done && o.final_tokens.is_empty());
        self.total_rounds += o.rounds as u128;
        self.total_commands += o.commands_executed as u128;
        self.max_rounds_seen = self.max_rounds_seen.max(o.rounds);
        self.max_tokens_seen = self.max_tokens_seen.max(o.max_tokens_seen);
        *self.histogram.entry((o.status, o.rounds)).or_insert(0) += 1;
        let small = OutcomeSummary::from(o);
        let slot = &mut self.examples[o.status.index()];
        if slot.as_ref().is_none_or(|e| sample_score(&e.outcome) < sample_score(&small)) {
            *slot = Some(Example { input: input.clone(), outcome: small.clone() });
        }
        if self.largest.as_ref().is_none_or(|e| e.outcome.max_tokens_seen < small.max_tokens_seen) {
            self.largest = Some(Example { input, outcome: small });
        }
    }
    fn merge(&mut self, other: Stats) {
        self.cases += other.cases;
        for i in 0..4 { self.statuses[i] += other.statuses[i]; }
        self.done_empty += other.done_empty;
        self.total_rounds += other.total_rounds;
        self.total_commands += other.total_commands;
        self.max_rounds_seen = self.max_rounds_seen.max(other.max_rounds_seen);
        self.max_tokens_seen = self.max_tokens_seen.max(other.max_tokens_seen);
        for (k, v) in other.histogram { *self.histogram.entry(k).or_insert(0) += v; }
        for (i, candidate) in other.examples.into_iter().enumerate() {
            if let Some(candidate) = candidate {
                if self.examples[i].as_ref().is_none_or(|e| sample_score(&e.outcome) < sample_score(&candidate.outcome)) {
                    self.examples[i] = Some(candidate);
                }
            }
        }
        if let Some(candidate) = other.largest {
            if self.largest.as_ref().is_none_or(|e| e.outcome.max_tokens_seen < candidate.outcome.max_tokens_seen) {
                self.largest = Some(candidate);
            }
        }
    }
}

#[derive(Clone)]
struct Config {
    min_length: usize, max_length: usize, max_rounds: usize, max_tokens: usize,
    threads: usize, max_cases: u64, sample_size: Option<u64>, seed: u64, out_dir: PathBuf,
}
impl Default for Config {
    fn default() -> Self {
        Self { min_length: 1, max_length: 8, max_rounds: 30, max_tokens: 10_000,
            threads: thread::available_parallelism().map_or(1, |n| n.get()),
            max_cases: 20_000_000, sample_size: None, seed: 42, out_dir: PathBuf::from("results") }
    }
}
fn parse_config(args: &[String]) -> Result<Config, String> {
    let mut c = Config::default();
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].as_str();
        i += 1;
        if i == args.len() { return Err(format!("missing value for {flag}")); }
        let value = args[i].as_str();
        match flag {
            "--min-length" => c.min_length = value.parse().map_err(|_| "invalid min-length")?,
            "--max-length" => c.max_length = value.parse().map_err(|_| "invalid max-length")?,
            "--max-rounds" => c.max_rounds = value.parse().map_err(|_| "invalid max-rounds")?,
            "--max-tokens" => c.max_tokens = value.parse().map_err(|_| "invalid max-tokens")?,
            "--threads" => c.threads = value.parse().map_err(|_| "invalid threads")?,
            "--max-cases" => c.max_cases = value.parse().map_err(|_| "invalid max-cases")?,
            "--sample-size" => c.sample_size = Some(value.parse().map_err(|_| "invalid sample-size")?),
            "--seed" => c.seed = value.parse().map_err(|_| "invalid seed")?,
            "--out-dir" => c.out_dir = PathBuf::from(value),
            _ => return Err(format!("unknown flag: {flag}")),
        }
        i += 1;
    }
    if c.min_length == 0 || c.max_length < c.min_length || c.max_length > 31 ||
       c.max_rounds == 0 || c.max_tokens == 0 || c.threads == 0 || c.max_cases == 0 ||
       c.sample_size == Some(0) {
        return Err("length must be 1..31; maximum >= minimum; rounds, tokens, threads, cases and samples must be > 0".into());
    }
    if c.min_length > c.max_tokens { return Err("initial length exceeds max-tokens".into()); }
    Ok(c)
}
fn csv_field(s: &str) -> String { format!("\"{}\"", s.replace('"', "\"\"")) }
fn write_line(w: &mut impl Write, fields: &[String]) -> std::io::Result<()> {
    writeln!(w, "{}", fields.join(","))
}
fn trace(input: &[u8], limits: Limits) {
    println!("ROUND 0: {}", format_tokens(input));
    let mut current = input.to_vec();
    let mut seen = vec![current.clone()];
    for round in 1..=limits.max_rounds {
        let (size, commands) = next_size(&current);
        if commands == 0 { println!("DONE: no commands ({} rounds)", round - 1); return; }
        if size > limits.max_tokens {
            println!("TOKEN LIMIT: next round requires {} tokens (max {})", size, limits.max_tokens); return;
        }
        let mut item = 0;
        for (i, &token) in current.iter().enumerate() {
            if !is_command(token) { continue; }
            item += 1;
            let suffix = &current[i+1..];
            let transformed: Vec<_> = if token == 0 { suffix.to_vec() } else { suffix.iter().map(|x| x ^ 1).collect() };
            println!("  cmd {item}: {} (at {}) | [{}] -> [{}]", NAMES[token as usize], i + 1,
                preview(suffix), preview(&transformed));
        }
        let next = step(&current, size);
        println!("ROUND {round}: {}", preview(&next));
        if !next.iter().any(|&t| is_command(t)) { println!("DONE: no commands ({} rounds)", round); return; }
        if let Some(start) = seen.iter().position(|s| *s == next) {
            println!("CYCLE: starts at round {}, period {}", start, round - start); return;
        }
        if round == limits.max_rounds { println!("ROUND LIMIT: {} rounds", round); return; }
        seen.push(next.clone());
        current = next;
    }
}
fn sweep(c: Config) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&c.out_dir)?;
    let mut summary = BufWriter::new(File::create(c.out_dir.join("summary.csv"))?);
    let mut hist = BufWriter::new(File::create(c.out_dir.join("rounds.csv"))?);
    let mut examples = BufWriter::new(File::create(c.out_dir.join("examples.csv"))?);
    writeln!(summary, "length,mode,cases,done,cycle,token_limit,round_limit,done_empty,done_rate,cycle_rate,mean_rounds,max_rounds_observed,max_tokens_observed,total_commands")?;
    writeln!(hist, "length,status,rounds,cases")?;
    writeln!(examples, "length,label,status,input,rounds,cycle_period,max_tokens_seen,final_tokens,final_preview")?;
    let limits = Limits { max_rounds:c.max_rounds, max_tokens:c.max_tokens };
    println!("Limits: {} rounds, {} tokens. Workers: {}. Seed: {}", c.max_rounds, c.max_tokens, c.threads, c.seed);
    for length in c.min_length..=c.max_length {
        if length > c.max_tokens { return Err(format!("length {length} exceeds --max-tokens").into()); }
        let total = 4u64.checked_pow(length as u32).ok_or("length too large for u64")?;
        let (cases, mode) = match c.sample_size {
            Some(s) => (s, "sample"),
            None if total <= c.max_cases => (total, "exhaustive"),
            None => return Err(format!("length {length}: {total} patterns exceed --max-cases {}. Use --sample-size N or increase --max-cases deliberately.", c.max_cases).into()),
        };
        let worker_count = c.threads.min(cases as usize).max(1);
        let results: Vec<Stats> = thread::scope(|scope| {
            let handles: Vec<_> = (0..worker_count).map(|worker| {
                let start = cases * worker as u64 / worker_count as u64;
                let end = cases * (worker as u64 + 1) / worker_count as u64;
                let seed = c.seed;
                scope.spawn(move || {
                    let mut stats = Stats::default();
                    for index in start..end {
                        let code = if mode == "sample" {
                            splitmix64(seed ^ ((length as u64) << 48) ^ index) % total
                        } else { index };
                        let input = decode(code, length);
                        let o = simulate(&input, limits);
                        stats.add(input, o);
                    }
                    stats
                })
            }).collect();
            handles.into_iter().map(|h| h.join().expect("worker panicked")).collect()
        });
        let mut stats = Stats::default();
        for part in results { stats.merge(part); }
        assert_eq!(stats.cases, cases);
        let values = [length.to_string(), mode.to_string(), stats.cases.to_string(),
            stats.statuses[0].to_string(),stats.statuses[1].to_string(),stats.statuses[2].to_string(),stats.statuses[3].to_string(),
            stats.done_empty.to_string(),format!("{:.8}", stats.statuses[0] as f64 / cases as f64),
            format!("{:.8}", stats.statuses[1] as f64 / cases as f64),
            format!("{:.4}",stats.total_rounds as f64 / cases as f64),
            stats.max_rounds_seen.to_string(),stats.max_tokens_seen.to_string(),stats.total_commands.to_string()];
        write_line(&mut summary, &values)?;
        for ((status, rounds), count) in stats.histogram {
            write_line(&mut hist, &[length.to_string(),status.as_str().into(),rounds.to_string(),count.to_string()])?;
        }
        for (label, sample) in STATUS_NAMES.iter().zip(stats.examples.into_iter())
            .chain(std::iter::once((&"largest", stats.largest))) {
            if let Some(example) = sample {
                let o = example.outcome;
                write_line(&mut examples, &[length.to_string(),label.to_string(),o.status.as_str().to_string(),
                    csv_field(&format_tokens(&example.input)),o.rounds.to_string(),
                    o.cycle_period.map_or(String::new(),|n|n.to_string()),o.max_tokens_seen.to_string(),
                    o.final_tokens.len().to_string(),csv_field(&preview(&o.final_tokens))])?;
            }
        }
        println!("len={length:>2} mode={mode:10} cases={cases:>12} done={:>9} cycle={:>9} token_cap={:>9} round_cap={:>9} max_len={}",
            stats.statuses[0], stats.statuses[1], stats.statuses[2], stats.statuses[3], stats.max_tokens_seen);
    }
    println!("CSV files saved under {}", c.out_dir.display());
    Ok(())
}
fn usage() {
    println!("ram-explorer — Repeat After Me Game research\n\n\
Usage:\n  ram-explorer trace 'RAM nRAM KON' [--max-rounds N --max-tokens N]\n  ram-explorer sweep [OPTIONS]\n\n\
Options (sweep):\n  --min-length N     first initial token length (default 1)\n  --max-length N     last initial token length (default 8)\n  --max-rounds N     stop after N completed rounds (default 30)\n  --max-tokens N     refuse next round exceeding N tokens (default 10000)\n  --threads N        worker threads (default logical CPU count)\n  --max-cases N      exhaustive per-length safety cap (default 20000000)\n  --sample-size N    random draws per length INSTEAD of exhaustive enumeration\n  --seed N           deterministic sampling seed (default 42)\n  --out-dir PATH     output directory (default results)\n\n\
Sampling uses replacement and can draw the same sequence more than once.\n\
Outputs: summary.csv, rounds.csv, examples.csv");
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("trace") => {
            if args.len() < 2 { return Err("trace requires an input string".into()); }
            let tokens = parse_tokens(&args[1])?;
            let limits = parse_config(&args[2..])?;
            trace(&tokens, Limits { max_rounds:limits.max_rounds, max_tokens:limits.max_tokens });
        }
        Some("sweep") => sweep(parse_config(&args[1..])?)?,
        Some("--help") | Some("-h") | None => usage(),
        _ => return Err("unknown command; use --help".into()),
    }
    Ok(())
}
fn main() {
    if let Err(error) = run() { eprintln!("Error: {error}"); std::process::exit(2); }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn run(s: &str) -> Outcome { simulate(&parse_tokens(s).unwrap(), Limits {max_rounds:30, max_tokens:10_000}) }
    #[test]
    fn whiteboard() {
        let input = parse_tokens("RAM nRAM KON").unwrap();
        let (size, _) = next_size(&input);
        let a = step(&input, size);
        assert_eq!(format_tokens(&a), "nRAM KON RAM");
        let (size, _) = next_size(&a);
        let b = step(&a, size);
        assert_eq!(format_tokens(&b), "RAM KON");
        let o = run("RAM nRAM KON");
        assert_eq!(o.status, Status::Done);
        assert_eq!(o.rounds, 3);
        assert_eq!(format_tokens(&o.final_tokens), "KON");
    }
    #[test]
    fn multiple_commands() {
        let o = run("RAM RAM KON");
        assert_eq!(o.status, Status::Done);
        assert_eq!(o.rounds, 2);
        assert_eq!(format_tokens(&o.final_tokens), "KON KON");
    }
    #[test]
    fn cycles() {
        let o = run("RAM RAM RAM");
        assert_eq!(o.status, Status::Cycle);
        assert_eq!(o.cycle_start, Some(0));
        assert_eq!(o.cycle_period, Some(1));
    }
    #[test]
    fn negation_and_empty() {
        assert_eq!(format_tokens(&run("nRAM nRAM").final_tokens), "nKON");
        let empty = run("RAM");
        assert_eq!(empty.status, Status::Done);
        assert_eq!(empty.rounds, 1);
        assert!(empty.final_tokens.is_empty());
        assert_eq!(run("KON nKON").rounds, 0);
    }
    #[test]
    fn guards() {
        let input = parse_tokens("RAM RAM RAM RAM").unwrap();
        let o = simulate(&input, Limits {max_rounds:30, max_tokens:5});
        assert_eq!(o.status, Status::TokenLimit);
        assert_eq!(o.rounds, 0);
        assert_eq!(o.attempted_tokens, Some(6));
        let o = simulate(&parse_tokens("RAM nRAM KON").unwrap(), Limits {max_rounds:1, max_tokens:100});
        assert_eq!(o.status, Status::RoundLimit);
        assert_eq!(o.rounds, 1);
    }
    #[test]
    fn base_four_encoding() {
        let list: Vec<String> = (0..16).map(|n| format_tokens(&decode(n,2))).collect();
        assert_eq!(list.first().unwrap(), "RAM RAM");
        assert_eq!(list.last().unwrap(), "nKON nKON");
        assert_eq!(list.len(), 16);
    }
}
