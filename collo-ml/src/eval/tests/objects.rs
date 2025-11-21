use crate::eval::{CheckedAST, EvalEnv, ExprValue, Object};
use crate::semantics::ExprType;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SimpleObject {
    Student1,
    Student2,
    Room1,
    Room2,
}

impl Object for SimpleObject {
    fn typ_name(&self) -> String {
        match self {
            Self::Student1 | Self::Student2 => "Student".into(),
            Self::Room1 | Self::Room2 => "Room".into(),
        }
    }

    fn field_access(&self, field: &str) -> ExprValue<Self> {
        match self {
            Self::Student1 => match field {
                "age" => ExprValue::Int(18),
                "enrolled" => ExprValue::Bool(true),
                _ => panic!("Invalid field for Student1"),
            },
            Self::Student2 => match field {
                "age" => ExprValue::Int(20),
                "enrolled" => ExprValue::Bool(false),
                _ => panic!("Invalid field for Student2"),
            },
            Self::Room1 => match field {
                "num" => ExprValue::Int(406),
                "students" => ExprValue::List(
                    ExprType::Object("Student".into()),
                    vec![ExprValue::Object(Self::Student1)],
                ),
                "first_student" => ExprValue::Object(Self::Student1),
                _ => panic!("Invalid field for Room1"),
            },
            Self::Room2 => match field {
                "num" => ExprValue::Int(406),
                "students" => ExprValue::List(
                    ExprType::Object("Student".into()),
                    vec![
                        ExprValue::Object(Self::Student1),
                        ExprValue::Object(Self::Student2),
                    ],
                ),
                "first_student" => ExprValue::Object(Self::Student2),
                _ => panic!("Invalid field for Room2"),
            },
        }
    }
}

fn eval_with_simple_objects(
    input: &str,
    fn_name: &str,
    args: Vec<ExprValue<SimpleObject>>,
) -> ExprValue<SimpleObject> {
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
    let types = HashMap::from([("Student".into(), student_type), ("Room".into(), room_type)]);
    let vars = HashMap::new();

    let checked_ast = CheckedAST::new(input, types, vars).expect("Should compile");
    let env = EvalEnv::from_objects([
        SimpleObject::Student1,
        SimpleObject::Student2,
        SimpleObject::Room1,
        SimpleObject::Room2,
    ])
    .expect("Types should be valid");

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
