use super::*;

#[test]
fn rejects_slot_over_midnight() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert_eq!(slot, None);
}

#[test]
fn rejects_slot_over_midnight_variant() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(22, 54, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(70).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert_eq!(slot, None);
}

#[test]
fn accepts_slot_within_day() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn accepts_slot_within_day_variant() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(94).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn accepts_slot_ending_at_midnight() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn accepts_slot_ending_at_midnight_variant() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(22, 32, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(88).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn accepts_slot_spanning_all_day() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(60 * 24).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn rejects_slot_ending_at_midnight_one_day_after() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(60 * 24 + 60).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_none());
}

#[test]
fn check_end_time_within_day() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.end_time(),
        chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap()
    );
}

#[test]
fn check_end_time_within_day_variant() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(94).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.end_time(),
        chrono::NaiveTime::from_hms_opt(8, 26, 0).unwrap()
    );
}

#[test]
fn check_end_time_at_midnight() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(22, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.end_time(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    );
}

#[test]
fn check_end_time_at_midnight_variant() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(22, 32, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(88).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.end_time(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    );
}

#[test]
fn check_end_time_for_slot_spanning_all_day() {
    let start = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(60 * 24).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.end_time(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    );
}

#[test]
fn check_non_overlapping() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(12, 52, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(12).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlap_with(&slot2));
}

#[test]
fn check_non_overlapping_order_inversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(12, 52, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(12).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot2.overlap_with(&slot1));
}

#[test]
fn check_non_overlapping_just_touching() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 26, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(12).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlap_with(&slot2));
}

#[test]
fn check_non_overlapping_just_touching_order_inversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(7, 34, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(52).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 26, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(68).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlap_with(&slot2));
}

#[test]
fn check_overlapping_not_included() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 12, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(95).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlap_with(&slot2));
}

#[test]
fn check_overlapping_not_included_order_inversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 12, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(95).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlap_with(&slot1));
}

#[test]
fn check_overlapping_included() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(7, 12, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlap_with(&slot2));
}

#[test]
fn check_overlapping_included_order_inversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(7, 12, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlap_with(&slot1));
}

#[test]
fn check_overlapping_barely() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 25, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlap_with(&slot2));
}

#[test]
fn check_overlapping_barely_order_inversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(8, 25, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlap_with(&slot1));
}

#[test]
fn check_overlapping_included_barely_front() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlap_with(&slot2));
}

#[test]
fn check_overlapping_included_barely_front_order_reversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlap_with(&slot1));
}

#[test]
fn check_overlapping_included_barely_back() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(7, 51, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlap_with(&slot2));
}

#[test]
fn check_overlapping_included_barely_back_order_reversed() {
    let start1 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        week: 1,
        weekday: chrono::Weekday::Wed.into(),
        start_time: chrono::NaiveTime::from_hms_opt(7, 51, 0).unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlap_with(&slot1));
}
