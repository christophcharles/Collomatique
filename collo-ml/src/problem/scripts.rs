#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StoredScript {
    script_ref: ScriptRef,
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

impl StoredScript {
    pub fn new(script: Script) -> Self {
        let script_ref = ScriptRef::new(script.name, &script.content);
        StoredScript {
            script_ref,
            content: script.content,
        }
    }

    pub fn get_ref(&self) -> &ScriptRef {
        &self.script_ref
    }

    pub fn get_content(&self) -> &str {
        &self.content
    }
}
