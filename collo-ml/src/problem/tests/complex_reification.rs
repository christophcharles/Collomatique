use crate::eval::ExprValue;
use crate::semantics::SimpleType;
use crate::traits::FieldConversionError;

use super::*;

use std::collections::BTreeSet;

// Simple EvalObject with 9 students
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum SimpleObject {
    Student1,
    Student2,
    Student3,
    Student4,
    Student5,
    Student6,
    Student7,
    Student8,
    Student9,
}

struct SimpleEnv {}

impl EvalObject for SimpleObject {
    type Env = SimpleEnv;
    type Cache = ();

    fn objects_with_typ(_env: &Self::Env, name: &str) -> BTreeSet<Self> {
        match name {
            "Student" => BTreeSet::from([
                SimpleObject::Student1,
                SimpleObject::Student2,
                SimpleObject::Student3,
                SimpleObject::Student4,
                SimpleObject::Student5,
                SimpleObject::Student6,
                SimpleObject::Student7,
                SimpleObject::Student8,
                SimpleObject::Student9,
            ]),
            _ => BTreeSet::new(),
        }
    }

    fn typ_name(&self, _env: &Self::Env) -> String {
        "Student".into()
    }

    fn type_id_to_name(_type_id: std::any::TypeId) -> Result<String, FieldConversionError> {
        // All our objects are Students
        Ok("Student".into())
    }

    fn field_access(
        &self,
        _env: &Self::Env,
        _cache: &mut Self::Cache,
        field: &str,
    ) -> Option<ExprValue<Self>> {
        // Students have an 'id' field for convenience
        match field {
            "id" => {
                let id = match self {
                    SimpleObject::Student1 => 1,
                    SimpleObject::Student2 => 2,
                    SimpleObject::Student3 => 3,
                    SimpleObject::Student4 => 4,
                    SimpleObject::Student5 => 5,
                    SimpleObject::Student6 => 6,
                    SimpleObject::Student7 => 7,
                    SimpleObject::Student8 => 8,
                    SimpleObject::Student9 => 9,
                };
                Some(ExprValue::Int(id))
            }
            _ => None,
        }
    }

    fn type_schemas() -> HashMap<String, HashMap<String, ExprType>> {
        let student_type = HashMap::from([("id".to_string(), SimpleType::Int.into())]);
        HashMap::from([("Student".into(), student_type)])
    }
}

// Variable system: StudentGroup takes a Student and returns a group number (1-3)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Var {
    StudentGroup(SimpleObject),
}

impl<T: EvalObject> EvalVar<T> for Var {
    fn field_schema() -> HashMap<String, Vec<crate::traits::FieldType>> {
        HashMap::from([(
            "StudentGroup".to_string(),
            vec![
                crate::traits::SimpleFieldType::Object(std::any::TypeId::of::<SimpleObject>())
                    .into(),
            ],
        )])
    }

    fn fix(&self, _env: &T::Env) -> Option<f64> {
        None // All students are valid, no need to fix
    }

    fn vars(
        _env: &T::Env,
    ) -> Result<std::collections::BTreeMap<Self, collomatique_ilp::Variable>, std::any::TypeId>
    {
        let mut vars = BTreeMap::new();
        // Create a variable for each student
        for student in [
            SimpleObject::Student1,
            SimpleObject::Student2,
            SimpleObject::Student3,
            SimpleObject::Student4,
            SimpleObject::Student5,
            SimpleObject::Student6,
            SimpleObject::Student7,
            SimpleObject::Student8,
            SimpleObject::Student9,
        ] {
            vars.insert(
                Var::StudentGroup(student),
                collomatique_ilp::Variable::integer().min(1.).max(3.),
            );
        }
        Ok(vars)
    }
}

impl TryFrom<&ExternVar<SimpleObject>> for Var {
    type Error = VarConversionError;
    fn try_from(value: &ExternVar<SimpleObject>) -> Result<Self, Self::Error> {
        match value.name.as_str() {
            "StudentGroup" => {
                if value.params.len() != 1 {
                    return Err(VarConversionError::WrongParameterCount {
                        name: "StudentGroup".into(),
                        expected: 1,
                        found: value.params.len(),
                    });
                }
                // Extract the student object from parameters
                let student = match &value.params[0] {
                    crate::eval::ExprValue::Object(obj) => SimpleObject::try_from(obj.clone())
                        .map_err(|_| VarConversionError::WrongParameterType {
                            name: "StudentGroup".into(),
                            param: 0,
                            expected: crate::traits::SimpleFieldType::Object(
                                std::any::TypeId::of::<SimpleObject>(),
                            )
                            .into(),
                        })?,
                    _ => {
                        return Err(VarConversionError::WrongParameterType {
                            name: "StudentGroup".into(),
                            param: 0,
                            expected: crate::traits::SimpleFieldType::Object(
                                std::any::TypeId::of::<SimpleObject>(),
                            )
                            .into(),
                        })
                    }
                };
                Ok(Var::StudentGroup(student))
            }
            _ => Err(VarConversionError::Unknown(value.name.clone())),
        }
    }
}
