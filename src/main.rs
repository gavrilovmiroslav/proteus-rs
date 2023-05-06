mod ast;

use lalrpop_util::lalrpop_mod;
lalrpop_mod!(pub proteus);

fn main() {
    println!("{:?}", proteus::ExprParser::new().parse("!(22 + \"qwe\" / x * foo(a, 2))"));
    println!("{:?}", proteus::ControlFlowParser::new().parse("int x = 5;"));
    println!("{:?}", proteus::ControlFlowParser::new().parse("x ! Foo(5);"));
    println!("{:?}", proteus::ControlFlowParser::new().parse("x = 7;"));
    println!("{:?}", proteus::ControlFlowParser::new().parse("Foo(1, x, \"qwe\");"));
    println!("{:?}", proteus::StateMachineParser::new().parse("state Foo { state A { on Foo(e) goto C {}; }; initial A; }"));
    println!("{:?}", proteus::TopLevelParser::new().parse("actor A {}"));
    println!("{:?}", proteus::TopLevelParser::new().parse("event PowerOn();"));
    println!("{:?}", proteus::TopLevelParser::new().parse("event Capture(int);"));
    println!("{:?}", proteus::TopLevelParser::new().parse("func add(int x, int y) -> int {}"));
    println!("{:?}", proteus::ProgramParser::new().parse("event PowerOn(); event PowerOff(); actor Lights {};"))
}
