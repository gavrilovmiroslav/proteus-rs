
#[derive(Debug)]
pub enum VarType {
    IntType,
    BoolType,
    FloatType,
    StringType,
}

pub type Program = Vec<TopLevelExpr>;

#[derive(Debug)]
pub enum TopLevelExpr {
    Actor { actor_name: String, content: Vec<ActorExpr> },
    Event { event_name: String, params: Vec<VarType> },
    Func { func_name: String, params: Vec<(String, VarType)>, ret_type: Option<VarType>, body: Vec<ControlFlowExpr> },
}

#[derive(Debug)]
pub enum ActorExpr {
    VarDecl { var_name: String, var_type: VarType, initial: Option<ValueExpr> },
    StateMachine(Vec<StateMachineExpr>),
    TransitionDecl { event: ValueExpr, conditions: Vec<ValueExpr>, body: Vec<ControlFlowExpr> },
    EntryDecl(Vec<ControlFlowExpr>),
    ExitDecl(Vec<ControlFlowExpr>),
}

#[derive(Debug)]
pub enum StateMachineExpr {
    VarDecl { var_name: String, var_type: VarType, initial: Option<ValueExpr> },
    InitialStateDecl(String),
    StateDecl { state_name: String, content: Vec<StateMachineExpr> },
    TransitionDecl { event: ValueExpr, conditions: Vec<ValueExpr>, target: String, body: Vec<ControlFlowExpr> },
    EntryDecl(Vec<ControlFlowExpr>),
    ExitDecl(Vec<ControlFlowExpr>),
}

#[derive(Debug)]
pub enum ControlFlowExpr {
    VarDecl { var_name: String, var_type: VarType, initial: Option<ValueExpr> },
    SendStatement { target_state: String, event: ValueExpr },
    AssignStatement { var_name: String, val_expr: ValueExpr },
    FuncCallStatement(ValueExpr),
}

#[derive(Debug)]
pub enum ValueExpr {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),
    AddExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    SubExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    MulExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    DivExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    NotExpr { v: Box<ValueExpr> },
    OrExpr  { l: Box<ValueExpr>, r: Box<ValueExpr> },
    AndExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    XorExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    EqExpr  { l: Box<ValueExpr>, r: Box<ValueExpr> },
    NeqExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    LeqExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    GeqExpr { l: Box<ValueExpr>, r: Box<ValueExpr> },
    LtExpr  { l: Box<ValueExpr>, r: Box<ValueExpr> },
    GtExpr  { l: Box<ValueExpr>, r: Box<ValueExpr> },
    FuncCallExpr { func_name: String, func_args: Vec<ValueExpr> },
}