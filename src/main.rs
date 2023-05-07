mod ast;
mod eval;
use crate::eval::{eval_program, EvalEngine};

use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub proteus);

fn main() {
    let mut engine = EvalEngine::default();

    engine.load_from_string(include_str!("program1.pro")).expect("Program loading failed");
    println!("{:#?}", engine);
}
