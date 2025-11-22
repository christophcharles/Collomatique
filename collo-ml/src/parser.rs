use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "collo-ml.pest"]
pub struct ColloMLParser;

#[cfg(test)]
mod tests;
