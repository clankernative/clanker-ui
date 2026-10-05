//! Exhaustive laws over a complete Gregorian 400-year cycle, plus domain edges.
use catalog_core::date::Date;

#[test]
fn complete_gregorian_cycle_roundtrips_orders_and_advances_weekdays() {
    let mut previous = None;
    for year in 1..=400 {
        let leap = year % 400 == 0 || (year % 4 == 0 && year % 100 != 0);
        let lengths = [
            31,
            if leap { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ];
        for (index, days) in lengths.into_iter().enumerate() {
            let month = index as u8 + 1;
            assert!(Date::new(year, month, 0).is_err());
            assert!(Date::new(year, month, days + 1).is_err());
            for day in 1..=days {
                let date = Date::new(year, month, day).unwrap();
                assert_eq!(Date::parse(&date.iso()).unwrap(), date);
                let encoded = serde_json::to_string(&date).unwrap();
                assert_eq!(serde_json::from_str::<Date>(&encoded).unwrap(), date);
                assert_eq!((date.year(), date.month(), date.day()), (year, month, day));
                if let Some(before) = previous {
                    assert!(date > before);
                    assert_eq!(before.add_days(1), Some(date));
                    assert_eq!(date.add_days(-1), Some(before));
                    assert_eq!(date.weekday(), (before.weekday() + 1) % 7);
                } else {
                    assert_eq!(date.weekday(), 1); // 0001-01-01 is Monday.
                }
                previous = Some(date);
            }
        }
    }
    assert_eq!(
        previous.unwrap().weekday(),
        Date::new(800, 12, 31).unwrap().weekday()
    );
}

#[test]
fn arithmetic_is_checked_for_extreme_inputs_and_domain_boundaries() {
    for date in [
        Date::new(1, 1, 1).unwrap(),
        Date::new(2024, 2, 29).unwrap(),
        Date::new(9999, 12, 31).unwrap(),
    ] {
        assert_eq!(date.add_months(i32::MIN), None);
        assert_eq!(date.add_months(i32::MAX), None);
        assert_eq!(date.add_days(0), Some(date));
        assert_eq!(date.add_months(0), Some(date));
    }
    assert_eq!(Date::new(1, 1, 1).unwrap().add_days(-1), None);
    assert_eq!(Date::new(9999, 12, 31).unwrap().add_days(1), None);
    assert_eq!(
        Date::new(2024, 1, 31).unwrap().add_months(1),
        Some(Date::new(2024, 2, 29).unwrap())
    );
    assert!(Date::new(0, 1, 1).is_err());
    assert!(Date::new(10000, 1, 1).is_err());
}
