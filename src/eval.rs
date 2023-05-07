use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use lockfree::queue::Queue;
use multimap::MultiMap;
use crate::ast::{ActorExpr, ControlFlowExpr, Program, StateMachineExpr, TopLevelExpr, ValueExpr, VarType};
use crate::proteus;

#[derive(Debug)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
}

#[derive(Debug)]
#[derive(Default)]
pub struct EventSignature {
    pub name: String,
    pub params: Vec<VarType>,
}

impl EventSignature {
    fn new(name: String, params: Vec<VarType>) -> EventSignature {
        EventSignature { name, params }
    }
}

#[derive(Debug)]
#[derive(Default)]
pub struct EventInstance {
    pub signature: EventSignature,
    pub params: Vec<Value>,
}

pub trait Inbox {
    fn poll(&mut self) -> Option<EventInstance>;
    fn push(&mut self, event: EventInstance);
}

pub trait Environment {
    fn set_var(&mut self, name: String, typ: VarType, initial: Value);
    fn get_var(&mut self, name: String) -> Option<&(VarType, Value)>;
}

/* ACTORS */

#[derive(Debug)]
#[derive(Default)]
pub struct Actor {
    pub name: String,
    pub queue: Queue<EventInstance>,
    pub env: HashMap<String, (VarType, Value)>,
    pub statemachine: Option<State>,
    pub transitions: MultiMap<String, Transition>,
}

impl Inbox for Actor {
    fn poll(&mut self) -> Option<EventInstance> {
        self.queue.pop()
    }

    fn push(&mut self, event: EventInstance) {
        self.queue.push(event);
    }
}

impl Environment for Actor {
    fn set_var(&mut self, name: String, typ: VarType, val: Value) {
        self.env.insert(name, (typ, val));
    }

    fn get_var(&mut self, name: String) -> Option<&(VarType, Value)> {
        self.env.get(name.as_str())
    }
}

/* STATE */

type ValueThunk = ValueExpr;
type Block = Vec<ControlFlowExpr>;

#[derive(Debug)]
#[derive(Default)]
pub struct Transition {
    pub event_name: String,
    pub bound_vars: Vec<String>,
    pub conditions: Vec<ValueThunk>,
    pub target: String,
    pub body: Block,
}

impl Transition {
    fn try_eval(func: ValueExpr, conditions: Vec<ValueExpr>, target: String, body: Block) -> Option<Transition> {
        return if let ValueExpr::FuncCallExpr { func_name, func_args } = func {
            let mut args = func_args.iter().map(|arg| {
                if let ValueExpr::Ident(id) = arg {
                    Some(id.clone())
                } else {
                    None
                }
            });

            if args.clone().any(|x| x.is_none()) {
                None
            } else {
                let arg_names = args.map(|x| x.unwrap().clone()).collect();
                Some(Transition {
                    event_name: func_name,
                    bound_vars: arg_names,
                    conditions,
                    target,
                    body
                })
            }
        } else {
            None
        }
    }
}

#[derive(Debug)]
#[derive(Default)]
pub struct State {
    pub name: String,
    pub at: String,
    pub env: HashMap<String, (VarType, Value)>,
    pub subs: HashMap<String, State>,
    pub transitions: MultiMap<String, Transition>,
}

impl Environment for State {
    fn set_var(&mut self, name: String, typ: VarType, val: Value) {
        self.env.insert(name, (typ, val));
    }

    fn get_var(&mut self, name: String) -> Option<&(VarType, Value)> {
        self.env.get(name.as_str())
    }
}

/* TOP LEVEL */

#[derive(Debug)]
#[derive(Default)]
pub struct EvalEngine {
    pub units: MultiMap<String, InterpretationUnit>,
}

impl EvalEngine {
    pub fn load_from_file(&mut self, filepath: &str) -> Result<(), String> {
        let name = filepath.to_string();
        let mut file = File::open(filepath).expect("Filepath doesn't exist");
        let mut content = String::new();
        file.read_to_string(&mut content).expect("Cannot read file to string");
        match proteus::ProgramParser::new().parse(&content) {
            Ok(ast) => {
                let unit = eval_program(name, ast);
                self.units.insert("".to_string(), unit);
                Ok(())
            }

            Err(err) => Err(format!("{:?}", err))
        }
    }

    pub fn load_from_string(&mut self, text: &str) -> Result<(), String> {
        match proteus::ProgramParser::new().parse(text) {
            Ok(ast) => {
                let unit = eval_program("".to_string(), ast);
                self.units.insert("".to_string(), unit);
                Ok(())
            }

            Err(err) => Err(format!("{:?}", err))
        }
    }
}

#[derive(Debug)]
#[derive(Default)]
pub struct InterpretationUnit {
    pub name: String,
    pub actors: HashMap<String, Actor>,
    pub events: HashMap<String, EventSignature>,
    pub funcs: HashMap<String, ()>,
}

impl InterpretationUnit {
    pub fn new(name: String) -> Self {
        InterpretationUnit {
            name: name.clone(),
            actors: HashMap::new(),
            events: HashMap::new(),
            funcs: HashMap::new(),
        }
    }
}

pub fn eval_pure(val: ValueExpr) -> Option<Value> {
    match val {
        ValueExpr::Bool(b) => Some(Value::Bool(b)),
        ValueExpr::Int(i) => Some(Value::Int(i)),
        ValueExpr::Float(f) => Some(Value::Float(f)),
        ValueExpr::Str(s) => Some(Value::Str(s)),
        _ => None
    }
}

pub fn eval_actor(name: String, content: Vec<ActorExpr>) -> Actor {
    let mut actor = Actor::default();
    actor.name = name.clone();
    for e in content {
        match e {
            ActorExpr::VarDecl { var_name, var_type, initial } => {
                actor.set_var(var_name, var_type, initial.map(|m| eval_pure(m).expect("Expected value, found expression.")).unwrap());
            }

            ActorExpr::StateMachine(sm) => {
                actor.statemachine = Some(State::default());
                eval_state(&String::default(), &mut actor.statemachine, sm);
            }

            ActorExpr::TransitionDecl { event, conditions, body } => {
                if let Some(trans) = Transition::try_eval(event, conditions, String::default(), body) {
                    actor.transitions.insert(trans.event_name.clone(), trans);
                } else {
                    println!("FAILED TO INTERPRET TRANSITION");
                }
            }

            ActorExpr::EntryDecl(body) => {
                actor.transitions.insert(String::from("_ENTRY"), Transition {
                    event_name: "_ENTRY".to_string(),
                    bound_vars: vec![],
                    conditions: vec![],
                    target: "".to_string(),
                    body
                })
            }

            ActorExpr::ExitDecl(body) => {
                actor.transitions.insert(String::from("_EXIT"), Transition {
                    event_name: "_EXIT".to_string(),
                    bound_vars: vec![],
                    conditions: vec![],
                    target: "".to_string(),
                    body
                })
            }
        }
    }

    actor
}

pub fn eval_state(name: &String, state: &mut Option<State>, sm: Vec<StateMachineExpr>) {
    state.as_mut().map(|state| {
        state.name = name.clone();

        for e in sm {
            match e {
                StateMachineExpr::VarDecl { var_name, var_type, initial } => {
                    state.set_var(var_name, var_type, initial.map(|m| eval_pure(m).expect("Expected value, found expression.")).unwrap());
                }

                StateMachineExpr::InitialStateDecl(state_name) => {
                    state.at = state_name;
                }

                StateMachineExpr::StateDecl { state_name, content } => {
                    let mut sub = Option::Some(State::default());
                    eval_state(&state_name, &mut sub, content);
                    state.subs.insert(state_name, sub.unwrap());
                }

                StateMachineExpr::TransitionDecl { event, conditions, target, body } => {
                    if let Some(trans) = Transition::try_eval(event, conditions, target, body) {
                        state.transitions.insert(trans.event_name.clone(), trans);
                    }
                }

                StateMachineExpr::EntryDecl(body) => {
                    state.transitions.insert(String::from("_ENTRY"), Transition {
                        event_name: "_ENTRY".to_string(),
                        bound_vars: vec![],
                        conditions: vec![],
                        target: "".to_string(),
                        body
                    })
                }

                StateMachineExpr::ExitDecl(body) => {
                    state.transitions.insert(String::from("_EXIT"), Transition {
                        event_name: "_EXIT".to_string(),
                        bound_vars: vec![],
                        conditions: vec![],
                        target: "".to_string(),
                        body
                    })
                }
            }
        }
    });
}

pub fn eval_program(name: String, program: Program) -> InterpretationUnit {
    let mut unit = InterpretationUnit::new(name);

    for e in program {
        match e {
            TopLevelExpr::Actor { actor_name, content } => {
                unit.actors.insert(actor_name.clone(), eval_actor(actor_name, content));
            }

            TopLevelExpr::Event { event_name, params } => {
                unit.events.insert(event_name.clone(), EventSignature::new(event_name, params));
            }

            TopLevelExpr::Func { func_name, params, ret_type, body } => {
                unit.funcs.insert(func_name.clone(), ());
            }
        }
    }

    unit
}