use std::env;
use std::process::exit;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: utt <op> <text>");
        exit(1);
    }

    let text = &args[2];

    let s = match args[1].to_lowercase().as_str() {
        "reverse" => reverse(text),
        "invert" => invert(text),
        "uppercase" => uppercase(text),
        "no-spaces" => no_spaces(text),
        "leet" => leet(text),
        "acronym" => acronym(text),
        op => {
            eprintln!("Invalid operation: {op}");
            exit(1);
        }
    };

    println!("Result: {s}");
}

fn reverse(s: &str) -> String {
    s.chars().rev().collect()
}

fn invert(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_uppercase() {
                c.to_lowercase().to_string()
            } else {
                c.to_uppercase().to_string()
            }
        })
        .collect()
}

fn uppercase(s: &str) -> String {
    s.to_uppercase()
}

fn no_spaces(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn leet(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A' | 'a' => '4',
            'B' | 'b' => '8',
            'E' | 'e' => '3',
            'G' | 'g' => '6',
            'I' | 'i' => '1',
            'L' | 'l' => '1',
            'O' | 'o' => '0',
            'S' | 's' => '5',
            'T' | 't' => '7',
            'Z' | 'z' => '2',
            _ => c,
        })
        .collect()
}

fn acronym(s: &str) -> String {
    s.split_whitespace()
        .filter_map(|word| word.chars().next())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}
