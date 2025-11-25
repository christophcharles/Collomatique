use super::*;

// ============================================================================
// Construction tests - midnight boundary behavior
// ============================================================================

#[test]
fn new_rejects_slot_crossing_midnight() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert_eq!(slot, None);
}

#[test]
fn new_rejects_slot_crossing_midnight_by_one_minute() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(23, 59, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(2).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert_eq!(slot, None);
}

#[test]
fn new_rejects_slot_crossing_midnight_variant() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(22, 54, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(70).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert_eq!(slot, None);
}

#[test]
fn new_accepts_slot_ending_exactly_at_midnight() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(22, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn new_accepts_slot_ending_exactly_at_midnight_variant() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(22, 32, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(88).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn new_accepts_full_24_hour_slot() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(60 * 24).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn new_rejects_slot_25_hours_long() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(23, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(60 * 24 + 60).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_none());
}

// ============================================================================
// Construction tests - normal cases
// ============================================================================

#[test]
fn new_accepts_normal_morning_slot() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn new_accepts_slot_with_odd_times() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(94).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

#[test]
fn new_accepts_very_short_slot() {
    let start = SlotStart {
        weekday: chrono::Weekday::Mon.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(1).unwrap();

    let slot = SlotWithDuration::new(start, duration);

    assert!(slot.is_some());
}

// ============================================================================
// End time calculation tests
// ============================================================================

#[test]
fn naive_end_time_calculates_correctly_for_morning_slot() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.naive_end_time(),
        chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap()
    );
}

#[test]
fn naive_end_time_calculates_correctly_with_odd_duration() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(94).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.naive_end_time(),
        chrono::NaiveTime::from_hms_opt(8, 26, 0).unwrap()
    );
}

#[test]
fn naive_end_time_returns_midnight_when_slot_ends_at_midnight() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(22, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(120).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.naive_end_time(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    );
}

#[test]
fn naive_end_time_returns_midnight_variant() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(22, 32, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(88).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.naive_end_time(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    );
}

#[test]
fn naive_end_time_for_full_day_slot() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(60 * 24).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(
        slot.naive_end_time(),
        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
    );
}

#[test]
fn end_time_returns_whole_minute_time() {
    let start = SlotStart {
        weekday: chrono::Weekday::Mon.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(90).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    let end = slot.end_time();
    assert_eq!(
        *end.inner(),
        chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap()
    );
}

// ============================================================================
// Accessor tests
// ============================================================================

#[test]
fn start_returns_slot_start() {
    let start = SlotStart {
        weekday: chrono::Weekday::Tue.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 15, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(45).unwrap();

    let slot = SlotWithDuration::new(start.clone(), duration).unwrap();

    assert_eq!(slot.start(), &start);
}

#[test]
fn duration_returns_correct_duration() {
    let start = SlotStart {
        weekday: chrono::Weekday::Fri.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 0, 0).unwrap())
            .unwrap(),
    };

    let duration = NonZeroDurationInMinutes::new(75).unwrap();

    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(slot.duration(), duration);
}

// ============================================================================
// Overlap tests - non-overlapping cases
// ============================================================================

#[test]
fn overlaps_with_returns_false_for_separate_slots() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(12, 52, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(12).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_false_for_separate_slots_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(12, 52, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(12).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot2.overlaps_with(&slot1));
}

#[test]
fn overlaps_with_returns_false_for_adjacent_slots() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 26, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(12).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_false_for_adjacent_slots_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(7, 34, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(52).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 26, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(68).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_false_for_different_weekdays() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Mon.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(120).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Tue.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(120).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(!slot1.overlaps_with(&slot2));
}

// ============================================================================
// Overlap tests - overlapping cases
// ============================================================================

#[test]
fn overlaps_with_returns_true_for_partial_overlap() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 12, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(95).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_true_for_partial_overlap_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 12, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(95).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlaps_with(&slot1));
}

#[test]
fn overlaps_with_returns_true_when_one_contains_other() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(7, 12, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_true_when_one_contains_other_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(7, 12, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlaps_with(&slot1));
}

#[test]
fn overlaps_with_returns_true_for_one_minute_overlap() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 25, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_true_for_one_minute_overlap_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 25, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlaps_with(&slot1));
}

#[test]
fn overlaps_with_returns_true_when_slots_start_at_same_time() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_true_when_slots_start_at_same_time_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlaps_with(&slot1));
}

#[test]
fn overlaps_with_returns_true_when_second_slot_ends_at_first_slot_end() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(7, 51, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot1.overlaps_with(&slot2));
}

#[test]
fn overlaps_with_returns_true_when_second_slot_ends_at_first_slot_end_reversed_order() {
    let start1 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(6, 52, 0).unwrap())
            .unwrap(),
    };
    let duration1 = NonZeroDurationInMinutes::new(94).unwrap();
    let slot1 = SlotWithDuration::new(start1, duration1).unwrap();

    let start2 = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(7, 51, 0).unwrap())
            .unwrap(),
    };
    let duration2 = NonZeroDurationInMinutes::new(35).unwrap();
    let slot2 = SlotWithDuration::new(start2, duration2).unwrap();

    assert!(slot2.overlaps_with(&slot1));
}

// ============================================================================
// Display and formatting tests
// ============================================================================

#[test]
fn display_formatting() {
    let start = SlotStart {
        weekday: chrono::Weekday::Mon.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap())
            .unwrap(),
    };
    let duration = NonZeroDurationInMinutes::new(90).unwrap();
    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(slot.to_string(), "lundi 09h30-11h00");
}

#[test]
fn display_formatting_sunday() {
    let start = SlotStart {
        weekday: chrono::Weekday::Sun.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap())
            .unwrap(),
    };
    let duration = NonZeroDurationInMinutes::new(60).unwrap();
    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(slot.to_string(), "Dimanche 14h00-15h00");
}

#[test]
fn capitalize_formatting() {
    let start = SlotStart {
        weekday: chrono::Weekday::Wed.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(10, 15, 0).unwrap())
            .unwrap(),
    };
    let duration = NonZeroDurationInMinutes::new(45).unwrap();
    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(slot.capitalize(), "Mercredi 10h15-11h00");
}

#[test]
fn capitalize_vs_display() {
    let start = SlotStart {
        weekday: chrono::Weekday::Tue.into(),
        start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap())
            .unwrap(),
    };
    let duration = NonZeroDurationInMinutes::new(120).unwrap();
    let slot = SlotWithDuration::new(start, duration).unwrap();

    assert_eq!(slot.capitalize(), "Mardi 08h00-10h00");
    assert_eq!(slot.to_string(), "mardi 08h00-10h00");
}

// ============================================================================
// Ordering tests
// ============================================================================

#[test]
fn ordering_by_weekday() {
    let monday_slot = SlotWithDuration::new(
        SlotStart {
            weekday: chrono::Weekday::Mon.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap())
                .unwrap(),
        },
        NonZeroDurationInMinutes::new(60).unwrap(),
    )
    .unwrap();

    let tuesday_slot = SlotWithDuration::new(
        SlotStart {
            weekday: chrono::Weekday::Tue.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(9, 0, 0).unwrap())
                .unwrap(),
        },
        NonZeroDurationInMinutes::new(60).unwrap(),
    )
    .unwrap();

    assert!(monday_slot < tuesday_slot);
}

#[test]
fn ordering_by_time_same_weekday() {
    let morning_slot = SlotWithDuration::new(
        SlotStart {
            weekday: chrono::Weekday::Wed.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap())
                .unwrap(),
        },
        NonZeroDurationInMinutes::new(120).unwrap(),
    )
    .unwrap();

    let afternoon_slot = SlotWithDuration::new(
        SlotStart {
            weekday: chrono::Weekday::Wed.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(14, 0, 0).unwrap())
                .unwrap(),
        },
        NonZeroDurationInMinutes::new(90).unwrap(),
    )
    .unwrap();

    assert!(morning_slot < afternoon_slot);
}

#[test]
fn equality() {
    let slot1 = SlotWithDuration::new(
        SlotStart {
            weekday: chrono::Weekday::Thu.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(11, 30, 0).unwrap())
                .unwrap(),
        },
        NonZeroDurationInMinutes::new(75).unwrap(),
    )
    .unwrap();

    let slot2 = SlotWithDuration::new(
        SlotStart {
            weekday: chrono::Weekday::Thu.into(),
            start_time: WholeMinuteTime::new(chrono::NaiveTime::from_hms_opt(11, 30, 0).unwrap())
                .unwrap(),
        },
        NonZeroDurationInMinutes::new(75).unwrap(),
    )
    .unwrap();

    assert_eq!(slot1, slot2);
}
