use gc::{Gc, Trace};
use std::ops::Deref;
use intern::{Symbol, SymbolIntern};
use std::collections::HashMap;
use {InterpError, Lambda};

#[derive(Clone)]
pub enum Value {
    List(Gc<Vec<Value>>),
    Map(Gc<MapWrapper>),
    String(Gc<String>),
    Float(f64),
    Int(i64),
    Bool(bool),
    Symbol(Symbol),
    Lambda(Gc<Lambda>),
}

#[derive(Debug)]
pub enum ValueKind {
    List,
    String,
    Float,
    Int,
    Bool,
    Symbol,
    Lambda,
}

#[derive(Debug, PartialEq)]
pub struct MapWrapper(HashMap<Value, Value>);

impl Deref for MapWrapper {
    type Target = HashMap<Value, Value>;
    fn deref(&self) -> &HashMap<Value, Value> {
        &self.0
    }
}

unsafe impl Trace for Value {
    custom_trace!(this, {
        match this {
            &Value::List(ref gc) => mark(gc),
            &Value::Map(ref gc) => mark(gc),
            &Value::String(ref gc) => mark(gc),
            &Value::Lambda(ref gc) => mark(gc),
            _ => {}
        }
    });
}

unsafe impl Trace for MapWrapper {
    custom_trace!(this, {
        for (k, v) in &this.0 {
            mark(k);
            mark(v);
        }
    });
}

impl ::std::fmt::Debug for Value {
    fn fmt(&self, formatter: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
        let empty_interner = SymbolIntern::new();
        let s = to_string_helper(self, &empty_interner);
        formatter.write_str(&s[..])
    }
}

impl Eq for Value {}
impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        use ::Value::*;

        match (self, other) {
            (&List(ref gc1), &List(ref gc2)) =>
                gc_to_usize(gc1) == gc_to_usize(gc2) || &**gc1 == &**gc2,
            (&Map(ref gc1), &Map(ref gc2)) =>
                gc_to_usize(gc1) == gc_to_usize(gc2) || &**gc1 == &**gc2,
            (&String(ref gc1), &String(ref gc2)) =>
                &**gc1 == &**gc2,
            (&Float(f1), &Float(f2)) => f1 == f2,
            (&Int(i1), &Int(i2)) => i1 == i2,
            (&Bool(b1), &Bool(b2)) => b1 == b2,
            (&Symbol(ref id1), &Symbol(ref id2)) => id1 == id2,
            //(&Lambda(ref l1, b1), &Lambda(ref l2, b2)) => l1 == l2 && b1 == b2,
            _ => false,
        }
    }
}

impl Value {
    pub fn expect_list(self) -> Result<Gc<Vec<Value>>, InterpError> {
        match self {
            Value::List(list) => Ok(list),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::List,
            }),
        }
    }

    pub fn expect_map(self) -> Result<Gc<MapWrapper>, InterpError> {
        match self {
            Value::Map(map) => Ok(map),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::List,
            }),
        }
    }

    pub fn expect_string(self) -> Result<Gc<String>, InterpError> {
        match self {
            Value::String(string) => Ok(string),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::String,
            }),
        }
    }

    pub fn expect_float(self) -> Result<f64, InterpError> {
        match self {
            Value::Float(float) => Ok(float),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::Float,
            }),
        }
    }

    pub fn expect_int(self) -> Result<i64, InterpError> {
        match self {
            Value::Int(int) => Ok(int),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::Int,
            }),
        }
    }

    pub fn expect_bool(self) -> Result<bool, InterpError> {
        match self {
            Value::Bool(b) => Ok(b),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::Bool,
            }),
        }
    }

    pub fn expect_symbol(self) -> Result<Symbol, InterpError> {
        match self {
            Value::Symbol(symbol) => Ok(symbol),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::Symbol,
            }),
        }
    }

    pub fn expect_lambda(self) -> Result<Gc<Lambda>, InterpError> {
        match self {
            Value::Lambda(lambda) => Ok(lambda),
            other => Err(InterpError::MismatchedType {
                value: other,
                expected: ValueKind::Lambda,
            }),
        }
    }

    pub fn expect_list_ref(&self) -> Result<&Gc<Vec<Value>>, InterpError> {
        match self {
            &Value::List(ref list) => Ok(list),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::List,
            }),
        }
    }

    pub fn expect_map_ref(&self) -> Result<&Gc<MapWrapper>, InterpError> {
        match self {
            &Value::Map(ref map) => Ok(map),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::List,
            }),
        }
    }

    pub fn expect_string_ref(&self) -> Result<&Gc<String>, InterpError> {
        match self {
            &Value::String(ref string) => Ok(string),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::String,
            }),
        }
    }

    pub fn expect_float_ref(&self) -> Result<&f64, InterpError> {
        match self {
            &Value::Float(ref float) => Ok(float),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Float,
            }),
        }
    }

    pub fn expect_int_ref(&self) -> Result<&i64, InterpError> {
        match self {
            &Value::Int(ref int) => Ok(int),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Int,
            }),
        }
    }

    pub fn expect_bool_ref(&self) -> Result<&bool, InterpError> {
        match self {
            &Value::Bool(ref b) => Ok(b),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Bool,
            }),
        }
    }

    pub fn expect_symbol_ref(&self) -> Result<&Symbol, InterpError> {
        match self {
            &Value::Symbol(ref symbol) => Ok(symbol),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Symbol,
            }),
        }
    }

    pub fn expect_lambda_ref(&self) -> Result<&Gc<Lambda>, InterpError> {
        match self {
            &Value::Lambda(ref lambda) => Ok(lambda),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Lambda,
            }),
        }
    }

    pub fn expect_list_ref_mut(&mut self) -> Result<&mut Gc<Vec<Value>>, InterpError> {
        match self {
            &mut Value::List(ref mut list) => Ok(list),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::List,
            }),
        }
    }

    pub fn expect_map_ref_mut(&mut self) -> Result<&mut Gc<MapWrapper>, InterpError> {
        match self {
            &mut Value::Map(ref mut map) => Ok(map),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::List,
            }),
        }
    }

    pub fn expect_string_ref_mut(&mut self) -> Result<&mut Gc<String>, InterpError> {
        match self {
            &mut Value::String(ref mut string) => Ok(string),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::String,
            }),
        }
    }

    pub fn expect_float_ref_mut(&mut self) -> Result<&mut f64, InterpError> {
        match self {
            &mut Value::Float(ref mut float) => Ok(float),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Float,
            }),
        }
    }

    pub fn expect_int_ref_mut(&mut self) -> Result<&mut i64, InterpError> {
        match self {
            &mut Value::Int(ref mut int) => Ok(int),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Int,
            }),
        }
    }

    pub fn expect_bool_ref_mut(&mut self) -> Result<&mut bool, InterpError> {
        match self {
            &mut Value::Bool(ref mut b) => Ok(b),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Bool,
            }),
        }
    }

    pub fn expect_symbol_ref_mut(&mut self) -> Result<&mut Symbol, InterpError> {
        match self {
            &mut Value::Symbol(ref mut symbol) => Ok(symbol),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Symbol,
            }),
        }
    }

    pub fn expect_lambda_ref_mut(&mut self) -> Result<&mut Gc<Lambda>, InterpError> {
        match self {
            &mut Value::Lambda(ref mut lambda) => Ok(lambda),
            other => Err(InterpError::MismatchedType {
                value: other.clone(),
                expected: ValueKind::Lambda,
            }),
        }
    }
}

fn gc_to_usize<T: Trace>(gc: &Gc<T>) -> usize {
    use std::mem::transmute;
    let ptr_t: &T = &**gc;
    unsafe { transmute(ptr_t) }
}

pub fn to_string_helper(value: &Value, interner: &SymbolIntern) -> String {
    use std::collections::HashSet;
    match value {
        &Value::Int(i) => format!("{}", i),
        &Value::Float(f) => format!("{}", f),
        &Value::String(ref s) => (&**s).clone(),
        &Value::Bool(b) => format!("{}", b),
        &Value::Symbol(s) => format!("'{}", interner.lookup_or_anon(s)),
        &Value::Lambda(_) => format!("<lambda>"),

        &ref l@Value::List(_) | &ref l@Value::Map(_) => {
            fn format_singles(vec: &Gc<Vec<Value>>,
                              buf: &mut String,
                              seen: &mut HashSet<usize>,
                              interner: &SymbolIntern) {
                let ptr = gc_to_usize(vec);
                if seen.contains(&ptr) {
                    buf.push_str("...")
                } else {
                    seen.insert(ptr);
                    buf.push_str("[");
                    for v in vec.iter() {
                        build_buf(v, buf, seen, interner);
                        buf.push_str(", ");
                    }
                    // remove trailing comma and space
                    if vec.len() >= 1 {
                        buf.pop();
                        buf.pop();
                    }
                    buf.push_str("]");
                }
            }
            fn format_pairs(m: &Gc<MapWrapper>,
                            buf: &mut String,
                            seen: &mut HashSet<usize>,
                            interner: &SymbolIntern) {
                let ptr = gc_to_usize(m);
                if seen.contains(&ptr) {
                    buf.push_str("...")
                } else {
                    seen.insert(ptr);
                    buf.push_str("{");
                    for (k, v) in m.iter() {
                        build_buf(k, buf, seen, interner);
                        buf.push_str(", ");
                        build_buf(v, buf, seen, interner);
                    }
                    buf.push_str("}")
                }
            }
            fn build_buf(cur: &Value,
                         buf: &mut String,
                         seen: &mut HashSet<usize>,
                         interner: &SymbolIntern) {
                match cur {
                    &Value::List(ref v) => format_singles(v, buf, seen, interner),
                    &Value::Map(ref m) => format_pairs(m, buf, seen, interner),
                    other => buf.push_str(&to_string_helper(&other, interner)),
                }
            }
            let mut inner = String::new();
            let mut seen = HashSet::new();
            build_buf(&l, &mut inner, &mut seen, interner);
            inner
        }
    }
}

impl ::std::hash::Hash for Value {
    fn hash<H>(&self, state: &mut H)
        where H: ::std::hash::Hasher
    {
        use std::mem::transmute;
        match self {
            &Value::List(ref rc) => rc.hash(state),
            &Value::Map(ref rc) => {
                for (k, v) in rc.iter() {
                    k.hash(state);
                    v.hash(state);
                }
            },
            &Value::String(ref rc) => rc.hash(state),
            &Value::Float(f) => unsafe { state.write(&transmute::<_, [u8; 8]>(f)) },
            &Value::Int(i) => unsafe { state.write(&transmute::<_, [u8; 8]>(i)) },
            &Value::Bool(b) => {
                let byte = if b {
                    1
                } else {
                    0
                };
                state.write(&[byte])
            },
            &Value::Symbol(ref rc) => rc.hash(state),
            &Value::Lambda(_) => {
                state.write(&[])
            }
        }
    }
}