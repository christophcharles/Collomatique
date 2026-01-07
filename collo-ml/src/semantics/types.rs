use super::SemError;

use std::{collections::BTreeSet, ops::Deref};

#[cfg(test)]
mod tests;

/// Represents a type that appears in a sum type
///
/// These can be primitive types (Int, Bool, LinExpr, etc)
/// or objects or even lists
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SimpleType {
    Never,
    Int,
    Bool,
    None,
    LinExpr,
    Constraint,
    String,
    EmptyList,
    List(ExprType),
    Object(String),
    Tuple(Vec<ExprType>), // (Int, Bool), (Int, Bool, String), etc.
}

/// Represents a sum type (or a simple type if there is only one type in it)
///
/// Invariants:
/// - there is always at least one type in it
/// - no type is the sum is a subtype of another in the sum
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExprType {
    variants: BTreeSet<SimpleType>,
}

impl SimpleType {
    pub fn is_primitive_type(&self) -> bool {
        matches!(
            self,
            SimpleType::Int
                | SimpleType::Bool
                | SimpleType::LinExpr
                | SimpleType::Constraint
                | SimpleType::None
                | SimpleType::String
        )
    }

    pub fn is_none(&self) -> bool {
        matches!(self, SimpleType::None)
    }

    pub fn is_list(&self) -> bool {
        matches!(self, SimpleType::List(_) | SimpleType::EmptyList)
    }

    pub fn is_empty_list(&self) -> bool {
        matches!(self, SimpleType::EmptyList)
    }

    pub fn is_lin_expr(&self) -> bool {
        matches!(self, SimpleType::LinExpr)
    }

    pub fn is_int(&self) -> bool {
        matches!(self, SimpleType::Int)
    }

    pub fn is_bool(&self) -> bool {
        matches!(self, SimpleType::Bool)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, SimpleType::String)
    }

    pub fn is_constraint(&self) -> bool {
        matches!(self, SimpleType::Constraint)
    }

    pub fn is_list_of_constraints(&self) -> bool {
        match self {
            SimpleType::List(inner) => inner.is_constraint(),
            _ => false,
        }
    }

    pub fn get_inner_list_type(&self) -> Option<&ExprType> {
        match self {
            SimpleType::List(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn to_inner_list_type(self) -> Option<ExprType> {
        match self {
            SimpleType::List(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, SimpleType::Object(_))
    }

    pub fn get_inner_object_type(&self) -> Option<&String> {
        match self {
            SimpleType::Object(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn to_inner_object_type(self) -> Option<String> {
        match self {
            SimpleType::Object(typ) => Some(typ),
            _ => None,
        }
    }

    pub fn is_arithmetic(&self) -> bool {
        matches!(self, SimpleType::Int | SimpleType::LinExpr)
    }

    pub fn is_tuple(&self) -> bool {
        matches!(self, SimpleType::Tuple(_))
    }

    pub fn get_tuple_elements(&self) -> Option<&Vec<ExprType>> {
        match self {
            SimpleType::Tuple(elements) => Some(elements),
            _ => None,
        }
    }

    pub fn is_concrete(&self) -> bool {
        match self {
            SimpleType::List(inner) => inner.is_concrete(),
            SimpleType::Tuple(elements) => elements.iter().all(|e| e.is_concrete()),
            _ => true,
        }
    }

    pub fn into_concrete(self) -> Option<ConcreteType> {
        if !self.is_concrete() {
            return None;
        }
        Some(ConcreteType { simple_typ: self })
    }

    pub fn is_subtype_of(&self, other: &Self) -> bool {
        match (self, other) {
            // Most types are either equal or distinct
            (a, b) if a == b => true,
            // Never is a subtype of everything
            (SimpleType::Never, _) => true,
            // Empty lists are subtypes of all lists
            (SimpleType::EmptyList, SimpleType::List(_)) => true,
            // For lists we have to recursively check
            (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                // otherwise we defer to the sum types
                inner1.is_subtype_of(inner2)
            }
            // Tuples are covariant: (A, B) <: (A', B') if A <: A' and B <: B'
            (SimpleType::Tuple(elems1), SimpleType::Tuple(elems2)) => {
                elems1.len() == elems2.len()
                    && elems1
                        .iter()
                        .zip(elems2.iter())
                        .all(|(e1, e2)| e1.is_subtype_of(e2))
            }
            // For all other combination, it's not
            _ => false,
        }
    }

    pub fn can_convert_to(&self, target: &ConcreteType) -> bool {
        let target = target.inner();
        match (self, target) {
            // Can convert (and this is a no-op) if we have the same as target typ
            (a, b) if a == b => true,
            // Int can be converted to LinExpr
            (SimpleType::Int, SimpleType::LinExpr) => true,
            // Empty lists can be converted to any other list type
            (SimpleType::EmptyList, SimpleType::List(_)) => true,
            // For list, we have to do that recursively
            (SimpleType::List(inner1), SimpleType::List(inner2)) => {
                let inner2_simple = inner2
                    .as_simple()
                    .expect("target type should be concrete and so simple");
                let inner2_concrete = inner2_simple
                    .clone()
                    .into_concrete()
                    .expect("target type should be concrete");
                inner1.can_convert_to(&inner2_concrete)
            }
            // Tuples: element-wise conversion
            (SimpleType::Tuple(elems1), SimpleType::Tuple(elems2)) => {
                elems1.len() == elems2.len()
                    && elems1.iter().zip(elems2.iter()).all(|(e1, e2)| {
                        let e2_simple = e2
                            .as_simple()
                            .expect("target type should be concrete and so simple");
                        let e2_concrete = e2_simple
                            .clone()
                            .into_concrete()
                            .expect("target type should be concrete");
                        e1.can_convert_to(&e2_concrete)
                    })
            }
            // Anything can convert to String
            (_, SimpleType::String) => true,
            // Anything else: no conversion
            _ => false,
        }
    }

    pub fn overlaps_with(&self, other: &SimpleType) -> bool {
        match (self, other) {
            // Same primitive types always overlap
            (SimpleType::Int, SimpleType::Int)
            | (SimpleType::Bool, SimpleType::Bool)
            | (SimpleType::None, SimpleType::None)
            | (SimpleType::LinExpr, SimpleType::LinExpr)
            | (SimpleType::Constraint, SimpleType::Constraint)
            | (SimpleType::String, SimpleType::String) => true,

            // Same object type overlaps
            (SimpleType::Object(s_name), SimpleType::Object(o_name)) => s_name == o_name,

            // Never overlaps with everything: it is a subtype of everything
            (SimpleType::Never, _) | (_, SimpleType::Never) => true,

            // Tuples overlap if same arity and all elements overlap
            (SimpleType::Tuple(elems1), SimpleType::Tuple(elems2)) => {
                elems1.len() == elems2.len()
                    && elems1
                        .iter()
                        .zip(elems2.iter())
                        .all(|(e1, e2)| e1.overlaps_with(e2))
            }

            // Tuples don't overlap with non-tuples
            (SimpleType::Tuple(_), _) | (_, SimpleType::Tuple(_)) => false,

            // Different primitive types don't overlap
            (SimpleType::Int, _)
            | (_, SimpleType::Int)
            | (SimpleType::Bool, _)
            | (_, SimpleType::Bool)
            | (SimpleType::None, _)
            | (_, SimpleType::None)
            | (SimpleType::LinExpr, _)
            | (_, SimpleType::LinExpr)
            | (SimpleType::Constraint, _)
            | (_, SimpleType::Constraint)
            | (SimpleType::Object(_), _)
            | (_, SimpleType::Object(_))
            | (SimpleType::String, _)
            | (_, SimpleType::String) => false,

            // Lists all overlap: the empty list is an example of all types
            (SimpleType::EmptyList, SimpleType::EmptyList)
            | (SimpleType::List(_), SimpleType::EmptyList)
            | (SimpleType::EmptyList, SimpleType::List(_))
            | (SimpleType::List(_), SimpleType::List(_)) => true,
        }
    }
}

impl SimpleType {
    fn assert_invariant(&self) {
        match self {
            SimpleType::List(inner) => inner.assert_invariant(),
            SimpleType::Tuple(elements) => {
                for elem in elements {
                    elem.assert_invariant();
                }
            }
            _ => {}
        }
    }
}

impl std::fmt::Display for SimpleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SimpleType::Never => write!(f, "Never"),
            SimpleType::None => write!(f, "None"),
            SimpleType::Bool => write!(f, "Bool"),
            SimpleType::Int => write!(f, "Int"),
            SimpleType::LinExpr => write!(f, "LinExpr"),
            SimpleType::Constraint => write!(f, "Constraint"),
            SimpleType::String => write!(f, "String"),
            SimpleType::EmptyList => write!(f, "[]"),
            SimpleType::List(sub_type) => write!(f, "[{}]", sub_type),
            SimpleType::Object(typ) => write!(f, "{}", typ),
            SimpleType::Tuple(elements) => {
                let types: Vec<_> = elements.iter().map(|t| t.to_string()).collect();
                write!(f, "({})", types.join(", "))
            }
        }
    }
}

impl TryFrom<crate::ast::SimpleTypeName> for SimpleType {
    type Error = SemError;

    fn try_from(value: crate::ast::SimpleTypeName) -> Result<Self, Self::Error> {
        use crate::ast::SimpleTypeName;
        match value {
            SimpleTypeName::Never => Ok(SimpleType::Never),
            SimpleTypeName::None => Ok(SimpleType::None),
            SimpleTypeName::Bool => Ok(SimpleType::Bool),
            SimpleTypeName::Int => Ok(SimpleType::Int),
            SimpleTypeName::LinExpr => Ok(SimpleType::LinExpr),
            SimpleTypeName::Constraint => Ok(SimpleType::Constraint),
            SimpleTypeName::String => Ok(SimpleType::String),
            SimpleTypeName::Object(name) => Ok(SimpleType::Object(name)),
            SimpleTypeName::EmptyList => Ok(SimpleType::EmptyList),
            SimpleTypeName::List(inner) => Ok(SimpleType::List(inner.try_into()?)),
            SimpleTypeName::Tuple(elements) => {
                let converted: Vec<ExprType> = elements
                    .into_iter()
                    .map(|e| ExprType::try_from(e))
                    .collect::<Result<_, _>>()?;
                Ok(SimpleType::Tuple(converted))
            }
        }
    }
}

impl TryFrom<crate::ast::Spanned<crate::ast::TypeName>> for ExprType {
    type Error = SemError;

    fn try_from(value: crate::ast::Spanned<crate::ast::TypeName>) -> Result<Self, SemError> {
        if value.node.types.is_empty() {
            panic!("It should not be possible to form 0-length typenames");
        }
        let mut flattened = Vec::with_capacity(value.node.types.len());
        for typ in value.node.types {
            let inner_typ = SimpleType::try_from(typ.node.inner)?;
            let spanned_inner = crate::ast::Spanned::new(inner_typ, typ.span);
            match typ.node.maybe_count {
                0 => flattened.push(spanned_inner),
                1 => {
                    if spanned_inner.node.is_none() {
                        return Err(SemError::OptionMarkerOnNone(spanned_inner.span));
                    }
                    flattened.push(crate::ast::Spanned::new(
                        SimpleType::None,
                        spanned_inner.span.clone(),
                    ));
                    flattened.push(spanned_inner);
                }
                _ => {
                    return Err(SemError::MultipleOptionMarkers {
                        typ: spanned_inner.node,
                        span: spanned_inner.span,
                    });
                }
            };
        }
        use std::collections::BTreeMap;
        let mut span_map = BTreeMap::new();
        for spanned_typ in flattened {
            let current_span = spanned_typ.span.clone();
            let old_span_opt = span_map.insert(spanned_typ.node.clone(), spanned_typ.span);
            if let Some(old_span) = old_span_opt {
                return Err(SemError::MultipleTypeInSum {
                    typ: spanned_typ.node,
                    span1: current_span,
                    span2: old_span,
                    sum_span: value.span,
                });
            }
        }
        let variants: BTreeSet<_> = span_map.keys().cloned().collect();
        if let Some((variant1, variant2)) = ExprType::check_subtypes(&variants) {
            let span1 = span_map.remove(variant1).unwrap();
            let span2 = span_map.remove(variant2).unwrap();
            return Err(SemError::SubtypeAndTypePresent {
                typ1: variant1.clone(),
                span1,
                typ2: variant2.clone(),
                span2,
                sum_span: value.span,
            });
        }
        Ok(ExprType { variants }.assert_before_return())
    }
}

impl From<SimpleType> for ExprType {
    fn from(value: SimpleType) -> Self {
        ExprType::simple(value)
    }
}

impl std::fmt::Display for ExprType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let types: Vec<_> = self.variants.iter().map(|t| t.to_string()).collect();
        write!(f, "{}", types.join(" | "))
    }
}

impl ExprType {
    fn check_subtypes(variants: &BTreeSet<SimpleType>) -> Option<(&SimpleType, &SimpleType)> {
        for variant1 in variants {
            for variant2 in variants {
                if variant1 == variant2 {
                    continue;
                }
                if variant1.is_subtype_of(variant2) {
                    return Some((variant1, variant2));
                }
            }
        }
        None
    }

    fn clean_subtypes(variants: &mut BTreeSet<SimpleType>) {
        while let Some((variant1, _variant2)) = Self::check_subtypes(variants) {
            let v = variant1.clone();
            variants.remove(&v);
        }
    }

    fn assert_invariant(&self) {
        assert!(
            self.variants.len() >= 1,
            "ExprType should always have at least one variant"
        );
        if let Some((variant1, variant2)) = Self::check_subtypes(&self.variants) {
            panic!(
                "{} is a subtype of {} and both present in ExprType; this is forbidden",
                variant1, variant2,
            );
        }
        for variant in &self.variants {
            variant.assert_invariant();
        }
    }

    fn assert_before_return(self) -> Self {
        self.assert_invariant();
        self
    }
}

impl ExprType {
    pub fn simple(typ: SimpleType) -> ExprType {
        ExprType {
            variants: BTreeSet::from([typ]),
        }
        .assert_before_return()
    }

    pub fn maybe(typ: SimpleType) -> Option<ExprType> {
        if typ.is_none() {
            return None;
        }
        Some(
            ExprType {
                variants: BTreeSet::from([SimpleType::None, typ]),
            }
            .assert_before_return(),
        )
    }

    pub fn sum(types: impl IntoIterator<Item = SimpleType>) -> Option<Self> {
        let mut variants: BTreeSet<_> = types.into_iter().collect();

        if variants.is_empty() {
            return None;
        }
        Self::clean_subtypes(&mut variants);
        Some(Self { variants }.assert_before_return())
    }

    pub fn is_simple(&self) -> bool {
        assert!(
            self.variants.len() >= 1,
            "ExprType should always carry at least one type"
        );
        self.variants.len() == 1
    }

    pub fn as_simple(&self) -> Option<&SimpleType> {
        if !self.is_simple() {
            return None;
        }
        Some(
            self.variants
                .iter()
                .next()
                .expect("ExprType should always carry at least one type"),
        )
    }

    pub fn to_simple(self) -> Option<SimpleType> {
        if !self.is_simple() {
            return None;
        }
        Some(
            self.variants
                .into_iter()
                .next()
                .expect("ExprType should always carry at least one type"),
        )
    }

    pub fn is_primitive_type(&self) -> bool {
        self.as_simple()
            .map(|x| x.is_primitive_type())
            .unwrap_or(false)
    }

    pub fn is_concrete(&self) -> bool {
        assert!(
            self.variants.len() >= 1,
            "ExprType should always carry at least one type"
        );
        if self.variants.len() != 1 {
            return false;
        }
        let variant = self.variants.iter().next().unwrap();
        variant.is_concrete()
    }

    pub fn get_inner_list_type(&self) -> Option<ExprType> {
        let mut variants = vec![];
        for v in &self.variants {
            if let SimpleType::List(inner) = v {
                variants.extend(inner.variants.iter().cloned())
            }
        }
        if variants.is_empty() {
            None
        } else {
            ExprType::sum(variants)
        }
    }

    pub fn has_list(&self) -> bool {
        self.variants.iter().any(|x| x.is_list())
    }

    pub fn is_list(&self) -> bool {
        self.variants.iter().all(|x| x.is_list())
    }

    pub fn is_empty_list(&self) -> bool {
        self.as_simple().map(|x| x.is_empty_list()).unwrap_or(false)
    }

    pub fn is_none(&self) -> bool {
        self.as_simple().map(|x| x.is_none()).unwrap_or(false)
    }

    pub fn is_sum_of_objects(&self) -> bool {
        self.variants
            .iter()
            .all(|x| matches!(x, SimpleType::Object(_)))
    }

    pub fn get_inner_object_type(&self) -> Option<&String> {
        self.as_simple()
            .map(|x| x.get_inner_object_type())
            .flatten()
    }

    pub fn to_inner_object_type(self) -> Option<String> {
        self.to_simple().map(|x| x.to_inner_object_type()).flatten()
    }

    pub fn is_object(&self) -> bool {
        self.as_simple().map(|x| x.is_object()).unwrap_or(false)
    }

    pub fn contains(&self, typ: &SimpleType) -> bool {
        self.variants.iter().any(|x| x == typ)
    }

    pub fn is_lin_expr(&self) -> bool {
        self.as_simple().map(|x| x.is_lin_expr()).unwrap_or(false)
    }

    pub fn is_int(&self) -> bool {
        self.as_simple().map(|x| x.is_int()).unwrap_or(false)
    }

    pub fn is_bool(&self) -> bool {
        self.as_simple().map(|x| x.is_bool()).unwrap_or(false)
    }

    pub fn is_string(&self) -> bool {
        self.as_simple().map(|x| x.is_string()).unwrap_or(false)
    }

    pub fn is_constraint(&self) -> bool {
        self.as_simple().map(|x| x.is_constraint()).unwrap_or(false)
    }

    pub fn is_list_of_constraints(&self) -> bool {
        self.as_simple()
            .map(|x| x.is_list_of_constraints())
            .unwrap_or(false)
    }

    pub fn is_arithmetic(&self) -> bool {
        self.variants.iter().all(|x| x.is_arithmetic())
    }

    pub fn get_variants(&self) -> &BTreeSet<SimpleType> {
        &self.variants
    }

    pub fn into_variants(self) -> BTreeSet<SimpleType> {
        self.variants
    }

    pub fn is_subtype_of(&self, other: &Self) -> bool {
        for variant in &self.variants {
            if other.variants.iter().all(|x| !variant.is_subtype_of(x)) {
                return false;
            }
        }
        true
    }

    pub fn can_convert_to(&self, target: &ConcreteType) -> bool {
        self.variants.iter().all(|x| x.can_convert_to(target))
    }

    pub fn unify_with(&self, other: &ExprType) -> ExprType {
        Self::sum(self.variants.union(&other.variants).cloned())
            .expect("There should be at least one variant")
    }

    pub fn cross_check<F: FnMut(&SimpleType, &SimpleType) -> Result<SimpleType, SemError>>(
        &self,
        other: &ExprType,
        errors: &mut Vec<SemError>,
        mut f: F,
    ) -> Option<ExprType> {
        let mut variants = BTreeSet::new();
        for v1 in &self.variants {
            for v2 in &other.variants {
                match f(v1, v2) {
                    Ok(t) => {
                        variants.insert(t);
                    }
                    Err(e) => {
                        errors.push(e);
                        return None;
                    }
                }
            }
        }
        assert!(
            !variants.is_empty(),
            "There should be at least one variant in the output"
        );

        Self::sum(variants)
    }

    pub fn overlaps_with(&self, other: &ExprType) -> bool {
        for variant in &self.variants {
            if other.variants.iter().any(|o| variant.overlaps_with(o)) {
                return true;
            }
        }
        false
    }

    pub fn substract(&self, other: &ExprType) -> Option<ExprType> {
        let variants = self
            .variants
            .iter()
            .filter(|x| !other.variants.iter().any(|y| x.is_subtype_of(y)))
            .cloned();
        ExprType::sum(variants)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConcreteType {
    simple_typ: SimpleType,
}

impl ConcreteType {
    pub fn inner(&self) -> &SimpleType {
        &self.simple_typ
    }

    pub fn into_inner(self) -> SimpleType {
        self.simple_typ
    }
}

impl Deref for ConcreteType {
    type Target = SimpleType;

    fn deref(&self) -> &Self::Target {
        self.inner()
    }
}

impl std::fmt::Display for ConcreteType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner())
    }
}
