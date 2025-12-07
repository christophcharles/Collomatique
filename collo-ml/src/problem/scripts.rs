use std::collections::HashMap;

use crate::{semantics::ArgsType, CheckedAST, EvalObject};

use super::ProblemError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredScript<T: EvalObject> {
    script_ref: ScriptRef,
    ast: CheckedAST<T>,
    content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Script {
    pub name: String,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptRef {
    name: String,
    hash: String,
}

impl ScriptRef {
    pub fn new(name: String, content: &str) -> Self {
        let hash = Self::hash_script(content);

        ScriptRef { name, hash }
    }

    fn hash_script(script: &str) -> String {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        use sha2::{Digest, Sha256};

        let hash = Sha256::digest(script);
        STANDARD.encode(hash)
    }
}

impl<T: EvalObject> StoredScript<T> {
    pub fn new(script: Script, vars: HashMap<String, ArgsType>) -> Result<Self, ProblemError<T>> {
        let script_ref = ScriptRef::new(script.name, &script.content);
        let ast = CheckedAST::new(&script.content, vars)?;
        Ok(StoredScript {
            script_ref,
            ast,
            content: script.content,
        })
    }

    pub fn get_ref(&self) -> &ScriptRef {
        &self.script_ref
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }

    pub fn get_ast(&self) -> &CheckedAST<T> {
        &self.ast
    }

    pub fn script(&self) -> Script {
        Script {
            name: self.script_ref.name.clone(),
            content: self.content.clone(),
        }
    }
}
