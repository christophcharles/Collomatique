use super::*;

#[test]
fn simple_csv() {
    let test_text = r#""Name";"Column2";"Column3";"Original name"
"BRETON";"2.0";"ESH";1546
"JULIEN";"1.5";"HGG";1850
"DUPONT";"-1.2";"Maths";1950"#;

    let csv_content = Content::from_raw(test_text.as_bytes());

    let params = Params {
        has_headers: true,
        delimiter: b';',
    };

    let extracted = csv_content.extract(&params).unwrap();

    let expected_result = Extract {
        headers: Some(vec![
            String::from("Name"),
            String::from("Column2"),
            String::from("Column3"),
            String::from("Original name"),
        ]),
        lines: vec![
            vec![
                String::from("BRETON"),
                String::from("2.0"),
                String::from("ESH"),
                String::from("1546"),
            ],
            vec![
                String::from("JULIEN"),
                String::from("1.5"),
                String::from("HGG"),
                String::from("1850"),
            ],
            vec![
                String::from("DUPONT"),
                String::from("-1.2"),
                String::from("Maths"),
                String::from("1950"),
            ],
        ],
    };

    assert_eq!(extracted, expected_result);
}

#[test]
fn no_headers() {
    let test_text = r#""Name";"Column2";"Column3";"Original name"
"BRETON";"2.0";"ESH";1546
"JULIEN";"1.5";"HGG";1850
"DUPONT";"-1.2";"Maths";1950"#;

    let csv_content = Content::from_raw(test_text.as_bytes());

    let params = Params {
        has_headers: false,
        delimiter: b';',
    };

    let extracted = csv_content.extract(&params).unwrap();

    let expected_result = Extract {
        headers: None,
        lines: vec![
            vec![
                String::from("Name"),
                String::from("Column2"),
                String::from("Column3"),
                String::from("Original name"),
            ],
            vec![
                String::from("BRETON"),
                String::from("2.0"),
                String::from("ESH"),
                String::from("1546"),
            ],
            vec![
                String::from("JULIEN"),
                String::from("1.5"),
                String::from("HGG"),
                String::from("1850"),
            ],
            vec![
                String::from("DUPONT"),
                String::from("-1.2"),
                String::from("Maths"),
                String::from("1950"),
            ],
        ],
    };

    assert_eq!(extracted, expected_result);
}

#[test]
fn wrong_delimiter() {
    let test_text = r#""Name";"Column2";"Column3";"Original name"
"BRETON";"2.0";"ESH";1546
"JULIEN";"1.5";"HGG";1850
"DUPONT";"-1.2";"Maths";1950"#;

    let csv_content = Content::from_raw(test_text.as_bytes());

    let params = Params {
        has_headers: true,
        delimiter: b',',
    };

    let extracted = csv_content.extract(&params).unwrap();

    let expected_result = Extract {
        headers: Some(vec![String::from(
            r#"Name;"Column2";"Column3";"Original name""#,
        )]),
        lines: vec![
            vec![String::from(r#"BRETON;"2.0";"ESH";1546"#)],
            vec![String::from(r#"JULIEN;"1.5";"HGG";1850"#)],
            vec![String::from(r#"DUPONT;"-1.2";"Maths";1950"#)],
        ],
    };

    assert_eq!(extracted, expected_result);
}
