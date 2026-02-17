use super::*;

#[test]
fn basic_construction() {
    let slot_start = SlotStart {
        weekday: Weekday(chrono::Weekday::Mon),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
            .unwrap(),
    };

    assert_eq!(slot_start.weekday, Weekday(chrono::Weekday::Mon));
    assert_eq!(
        *slot_start.start_time.inner(),
        chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap()
    );
}

#[test]
fn display_formatting() {
    let slot_start = SlotStart {
        weekday: Weekday(chrono::Weekday::Mon),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 30, 0).unwrap())
            .unwrap(),
    };

    assert_eq!(slot_start.to_string(), "lundi 14h30");
}

#[test]
fn display_formatting_sunday() {
    let slot_start = SlotStart {
        weekday: Weekday(chrono::Weekday::Sun),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap())
            .unwrap(),
    };

    assert_eq!(slot_start.to_string(), "Dimanche 10h00");
}

#[test]
fn capitalize_formatting() {
    let slot_start = SlotStart {
        weekday: Weekday(chrono::Weekday::Tue),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 15, 0).unwrap())
            .unwrap(),
    };

    assert_eq!(slot_start.capitalize(), "Mardi 09h15");
}

#[test]
fn capitalize_vs_display() {
    let slot_start = SlotStart {
        weekday: Weekday(chrono::Weekday::Wed),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap())
            .unwrap(),
    };

    // capitalize should have capital W in Mercredi
    assert_eq!(slot_start.capitalize(), "Mercredi 11h00");
    // Display should have lowercase m
    assert_eq!(slot_start.to_string(), "mercredi 11h00");
}

#[test]
fn ordering_by_weekday() {
    let monday = SlotStart {
        weekday: Weekday(chrono::Weekday::Mon),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap())
            .unwrap(),
    };

    let tuesday = SlotStart {
        weekday: Weekday(chrono::Weekday::Tue),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap())
            .unwrap(),
    };

    assert!(monday < tuesday);
}

#[test]
fn ordering_by_time_same_weekday() {
    let morning = SlotStart {
        weekday: Weekday(chrono::Weekday::Mon),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
            .unwrap(),
    };

    let afternoon = SlotStart {
        weekday: Weekday(chrono::Weekday::Mon),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap())
            .unwrap(),
    };

    assert!(morning < afternoon);
}

#[test]
fn ordering_weekday_dominates() {
    // Even if Monday is later in the day, it should come before Tuesday morning
    let monday_afternoon = SlotStart {
        weekday: Weekday(chrono::Weekday::Mon),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(17, 0, 0).unwrap())
            .unwrap(),
    };

    let tuesday_morning = SlotStart {
        weekday: Weekday(chrono::Weekday::Tue),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap())
            .unwrap(),
    };

    assert!(monday_afternoon < tuesday_morning);
}

#[test]
fn equality() {
    let slot1 = SlotStart {
        weekday: Weekday(chrono::Weekday::Fri),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(13, 30, 0).unwrap())
            .unwrap(),
    };

    let slot2 = SlotStart {
        weekday: Weekday(chrono::Weekday::Fri),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(13, 30, 0).unwrap())
            .unwrap(),
    };

    assert_eq!(slot1, slot2);
}
