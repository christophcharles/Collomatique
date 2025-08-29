use super::*;

fn test_part_uppercase(part: &str) -> bool {
    part.chars()
        .filter(|c| c.is_alphabetic())
        .all(|c| c.is_uppercase())
}

fn compute_name_size_hint(
    parts: &Vec<&str>,
    first_name_first: bool,
) -> Option<std::num::NonZeroUsize> {
    if first_name_first {
        let mut count = 0usize;
        for part in parts.iter().rev() {
            let upper_case = test_part_uppercase(*part);
            if upper_case {
                count += 1
            } else {
                break;
            }
        }
        if count == 0 {
            return None;
        }
        let pos = parts.len() - count;
        std::num::NonZeroUsize::new(pos)
    } else {
        let mut count = 0usize;
        for part in parts {
            let upper_case = test_part_uppercase(*part);
            if upper_case {
                count += 1
            } else {
                break;
            }
        }
        if count == parts.len() {
            return None;
        }
        std::num::NonZeroUsize::new(count)
    }
}

fn extract_name_parts_with_hint(
    name_parts: &Vec<&str>,
    firstname_first: bool,
    hint: std::num::NonZeroUsize,
) -> (String, String) {
    let first_name;
    let last_name;
    if firstname_first {
        last_name = name_parts[hint.get()..].join(" ");
        first_name = name_parts[0..hint.get()].join(" ");
    } else {
        last_name = name_parts[0..hint.get()].join(" ");
        first_name = name_parts[hint.get()..].join(" ");
    }
    (first_name, last_name)
}

fn extract_name_parts_no_hint(name: &String, firstname_first: bool) -> (String, String) {
    let first_name;
    let last_name;

    if firstname_first {
        match name.rsplit_once(' ') {
            None => {
                last_name = name.clone();
                first_name = String::from("");
            }
            Some((firstname, surname)) => {
                last_name = surname.to_string();
                first_name = firstname.to_string();
            }
        }
    } else {
        match name.split_once(' ') {
            None => {
                last_name = name.clone();
                first_name = String::from("");
            }
            Some((surname, firstname)) => {
                last_name = surname.to_string();
                first_name = firstname.to_string();
            }
        }
    }

    (first_name, last_name)
}

#[pyfunction]
#[pyo3(signature = (name, firstname_first = true))]
pub fn extract_name_parts(name: String, firstname_first: bool) -> (String, String) {
    let parts = name.split(' ').collect();

    match compute_name_size_hint(&parts, firstname_first) {
        None => extract_name_parts_no_hint(&name, firstname_first),
        Some(hint) => extract_name_parts_with_hint(&parts, firstname_first, hint),
    }
}
