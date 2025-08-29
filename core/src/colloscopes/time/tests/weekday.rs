#[test]
fn mon_less_than_tue() {
    let day1: super::Weekday = chrono::Weekday::Mon.into();
    let day2: super::Weekday = chrono::Weekday::Tue.into();

    assert!(day1 < day2);
}

#[test]
fn tue_less_than_wed() {
    let day1: super::Weekday = chrono::Weekday::Tue.into();
    let day2: super::Weekday = chrono::Weekday::Wed.into();

    assert!(day1 < day2);
}

#[test]
fn wed_less_than_thu() {
    let day1: super::Weekday = chrono::Weekday::Wed.into();
    let day2: super::Weekday = chrono::Weekday::Thu.into();

    assert!(day1 < day2);
}

#[test]
fn thu_less_than_fri() {
    let day1: super::Weekday = chrono::Weekday::Thu.into();
    let day2: super::Weekday = chrono::Weekday::Fri.into();

    assert!(day1 < day2);
}

#[test]
fn fri_less_than_sat() {
    let day1: super::Weekday = chrono::Weekday::Fri.into();
    let day2: super::Weekday = chrono::Weekday::Sat.into();

    assert!(day1 < day2);
}

#[test]
fn sat_less_than_sun() {
    let day1: super::Weekday = chrono::Weekday::Sat.into();
    let day2: super::Weekday = chrono::Weekday::Sun.into();

    assert!(day1 < day2);
}

#[test]
fn sun_bigger_than_mon() {
    let day1: super::Weekday = chrono::Weekday::Sun.into();
    let day2: super::Weekday = chrono::Weekday::Mon.into();

    assert!(day1 > day2);
}
