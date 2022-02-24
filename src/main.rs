mod inst;
mod program;

fn main() {
    let file = std::env::args().nth(1).expect("required input file as argument argument");
    let contents =
        std::fs::read_to_string(&format!("asm/{}.asm", file)).expect("failed to open file");

    let prog = contents
        .parse::<program::Program>()
        .expect("failed to parse program");
    dbg!(prog);
}
