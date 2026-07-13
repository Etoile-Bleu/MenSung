//! A calendar date recording when a claim was last confirmed against its
//! source, e.g. when a drug label was last revised. Deliberately minimal:
//! no timezone, no arithmetic, just enough validation to reject an
//! impossible date and a stable `Display`/`Ord` for showing and sorting
//! claims by recency. A full date/time crate would be overkill for a value
//! this project only ever displays and compares, never computes with.

use crate::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClaimDate {
    year: u16,
    month: u8,
    day: u8,
}

impl ClaimDate {
    pub fn new(year: u16, month: u8, day: u8) -> Result<Self, DomainError> {
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(year) => 29,
            2 => 28,
            _ => 0,
        };

        if day == 0 || day > days_in_month {
            return Err(DomainError::InvalidClaimDate { year, month, day });
        }

        Ok(Self { year, month, day })
    }

    pub fn year(self) -> u16 {
        self.year
    }

    pub fn month(self) -> u8 {
        self.month
    }

    pub fn day(self) -> u8 {
        self.day
    }
}

fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

impl std::fmt::Display for ClaimDate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_valid_date() {
        let date = ClaimDate::new(2026, 7, 14).unwrap();
        assert_eq!(date.to_string(), "2026-07-14");
    }

    #[test]
    fn rejects_month_13() {
        assert!(ClaimDate::new(2026, 13, 1).is_err());
    }

    #[test]
    fn rejects_day_zero() {
        assert!(ClaimDate::new(2026, 1, 0).is_err());
    }

    #[test]
    fn rejects_february_30() {
        assert!(ClaimDate::new(2026, 2, 30).is_err());
    }

    #[test]
    fn accepts_february_29_on_a_leap_year() {
        assert!(ClaimDate::new(2024, 2, 29).is_ok());
    }

    #[test]
    fn rejects_february_29_on_a_non_leap_year() {
        assert!(ClaimDate::new(2026, 2, 29).is_err());
    }

    #[test]
    fn rejects_february_29_on_a_century_non_leap_year() {
        assert!(ClaimDate::new(1900, 2, 29).is_err());
    }

    #[test]
    fn accepts_february_29_on_a_four_century_leap_year() {
        assert!(ClaimDate::new(2000, 2, 29).is_ok());
    }

    #[test]
    fn dates_order_chronologically() {
        let earlier = ClaimDate::new(2025, 12, 31).unwrap();
        let later = ClaimDate::new(2026, 1, 1).unwrap();
        assert!(earlier < later);
    }
}
