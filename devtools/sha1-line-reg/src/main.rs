use std::io;

use linreg::linear_regression_of;
use regex::Regex;

/// convert amount/denom string to float in nano seconds
fn parse_str_as_float_ns(amount: &str, denom: &str) -> f64 {
    let a = amount.parse::<f64>().unwrap();
    a * match denom {
        "s" => 1e9,
        "ms" => 1e6,
        "µs" => 1e3,
        "ns" => 1f64,
        "ps" => 1e-3,
        _ => panic!("unexpected denom"),
    }
}

/// read inputed criterion benchmarks of sha1 and return vec of the pair
/// (blocks, cost time in nano seconds)
fn read_criterion() -> Vec<(f64, f64)> {
    let mut result: Vec<(f64, f64)> = vec![];

    let re = Regex::new(
        r"^sha1/blocks/(\d+)\s+time:\s+\[[\d.]+ [pµnm]?s ([\d.]+) ([pµnm]?s)+ [\d.]+ [pµnm]?s\].*$",
    )
    .unwrap();

    let lines = io::stdin().lines();
    for line in lines {
        if let Some(caps) = re.captures(&line.unwrap()) {
            let x = caps.get(1).unwrap().as_str().parse::<f64>().unwrap();
            let y =
                parse_str_as_float_ns(caps.get(2).unwrap().as_str(), caps.get(3).unwrap().as_str());
            result.push((x, y))
        }
    }

    result
}

/// read criterion benchmarks result from stdin and do linear regression
/// of (block, cost time)
fn main() {
    let benches = read_criterion();
    // skip if the block is 1 because it is always
    // outlier in benchmarks
    let (slope, intercept): (f64, f64) = linear_regression_of(&benches[1..]).unwrap();
    println!("slope: {} (ns/block), intercept: {} (ns)", slope, intercept);
    println!(
        "block1: {} * slope + intercept",
        (benches[0].1 - intercept) / slope
    )
}
