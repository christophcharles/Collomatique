use crate::eval::{CheckedAST, EvalObject, ExprValue};
use crate::semantics::ExprType;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SimpleObject {
    Student1,
    Student2,
    Room1,
    Room2,
}

struct SimpleEnv {}

impl EvalObject for SimpleObject {
    type Env = SimpleEnv;

    fn objects_with_typ(_env: &Self::Env, name: &str) -> BTreeSet<Self> {
        match name {
            "Student" => BTreeSet::from([SimpleObject::Student1, SimpleObject::Student2]),
            "Room" => BTreeSet::from([SimpleObject::Room1, SimpleObject::Room2]),
            _ => BTreeSet::new(),
        }
    }

    fn typ_name(&self, _env: &Self::Env) -> String {
        match self {
            SimpleObject::Student1 | SimpleObject::Student2 => "Student".into(),
            SimpleObject::Room1 | SimpleObject::Room2 => "Room".into(),
        }
    }

    fn field_access(&self, _env: &Self::Env, field: &str) -> Option<ExprValue<Self>> {
        match self {
            SimpleObject::Student1 => match field {
                "age" => Some(ExprValue::Int(18)),
                "enrolled" => Some(ExprValue::Bool(true)),
                _ => None,
            },
            SimpleObject::Student2 => match field {
                "age" => Some(ExprValue::Int(20)),
                "enrolled" => Some(ExprValue::Bool(false)),
                _ => None,
            },
            SimpleObject::Room1 => match field {
                "num" => Some(ExprValue::Int(406)),
                "students" => Some(ExprValue::List(
                    ExprType::Object("Student".into()),
                    BTreeSet::from([ExprValue::Object(SimpleObject::Student1)]),
                )),
                "first_student" => Some(ExprValue::Object(SimpleObject::Student1)),
                _ => None,
            },
            SimpleObject::Room2 => match field {
                "num" => Some(ExprValue::Int(406)),
                "students" => Some(ExprValue::List(
                    ExprType::Object("Student".into()),
                    BTreeSet::from([
                        ExprValue::Object(SimpleObject::Student1),
                        ExprValue::Object(SimpleObject::Student2),
                    ]),
                )),
                "first_student" => Some(ExprValue::Object(SimpleObject::Student2)),
                _ => None,
            },
        }
    }

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        let student_type = HashMap::from([
            ("age".to_string(), ExprType::Int),
            ("enrolled".to_string(), ExprType::Bool),
        ]);
        let room_type = HashMap::from([
            ("num".to_string(), ExprType::Int),
            (
                "students".to_string(),
                ExprType::List(Box::new(ExprType::Object("Student".into()))),
            ),
            (
                "first_student".to_string(),
                ExprType::Object("Student".into()),
            ),
        ]);

        HashMap::from([("Student".into(), student_type), ("Room".into(), room_type)])
    }
}

fn eval_with_simple_objects(
    input: &str,
    fn_name: &str,
    args: Vec<ExprValue<SimpleObject>>,
) -> ExprValue<SimpleObject> {
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, vars).expect("Should compile");
    let env = SimpleEnv {};

    checked_ast
        .eval_fn(&env, fn_name, args)
        .expect("Should evaluate")
}

#[test]
fn simple_object() {
    let input = "pub let f(s: Student) -> Student = s;";

    let args = vec![ExprValue::Object(SimpleObject::Student1)];

    let result = eval_with_simple_objects(input, "f", args);

    assert_eq!(result, ExprValue::Object(SimpleObject::Student1));
}

#[test]
fn simple_field_access() {
    let input = "pub let f(s: Student) -> Int = s.age;";

    let args = vec![ExprValue::Object(SimpleObject::Student1)];

    let result = eval_with_simple_objects(input, "f", args);

    assert_eq!(result, ExprValue::Int(18));
}

#[test]
fn nested_field_access() {
    let input = "pub let f(r: Room) -> Bool = r.first_student.enrolled;";

    let args = vec![ExprValue::Object(SimpleObject::Room1)];

    let result = eval_with_simple_objects(input, "f", args);

    assert_eq!(result, ExprValue::Bool(true));
}

#[test]
fn nested_field_access2() {
    let input = "pub let f(r: Room) -> Int = r.first_student.age;";

    let args = vec![ExprValue::Object(SimpleObject::Room2)];

    let result = eval_with_simple_objects(input, "f", args);

    assert_eq!(result, ExprValue::Int(20));
}

#[test]
fn global_list() {
    let input = "pub let f() -> [Student] = @[Student];";

    let result = eval_with_simple_objects(input, "f", vec![]);

    assert_eq!(
        result,
        ExprValue::List(
            ExprType::Object("Student".into()),
            BTreeSet::from([
                ExprValue::Object(SimpleObject::Student1),
                ExprValue::Object(SimpleObject::Student2),
            ])
        )
    );
}
