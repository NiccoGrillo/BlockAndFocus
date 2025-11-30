//! Schedule evaluation engine.

use blockandfocus_shared::{Schedule, ScheduleRule, WeekdayWrapper};
use chrono::{Datelike, Local, Timelike, Weekday};
use tracing::debug;

/// Engine for evaluating schedule rules.
pub struct ScheduleEngine {
    schedule: Schedule,
}

impl ScheduleEngine {
    /// Create a new schedule engine.
    pub fn new(schedule: Schedule) -> Self {
        Self { schedule }
    }

    /// Update the schedule configuration.
    pub fn update(&mut self, schedule: Schedule) {
        self.schedule = schedule;
    }

    /// Check if blocking should be active based on schedule.
    ///
    /// Returns true if:
    /// - Schedule is disabled (blocking always active), OR
    /// - Current time falls within any active schedule rule
    pub fn is_blocking_time(&self) -> bool {
        if !self.schedule.enabled {
            // Schedule disabled means blocking is always active
            return true;
        }

        if self.schedule.rules.is_empty() {
            // No rules means no scheduled blocking
            return false;
        }

        let now = Local::now();
        let current_day = now.weekday();
        let current_time = now.time();

        for rule in &self.schedule.rules {
            if self.rule_matches(rule, current_day, current_time) {
                debug!(
                    rule_name = %rule.name,
                    "Schedule rule active"
                );
                return true;
            }
        }

        false
    }

    /// Get the name of the currently active schedule rule (if any).
    pub fn active_rule_name(&self) -> Option<String> {
        if !self.schedule.enabled || self.schedule.rules.is_empty() {
            return None;
        }

        let now = Local::now();
        let current_day = now.weekday();
        let current_time = now.time();

        for rule in &self.schedule.rules {
            if self.rule_matches(rule, current_day, current_time) {
                return Some(rule.name.clone());
            }
        }

        None
    }

    /// Check if a specific rule matches the given day and time.
    fn rule_matches(
        &self,
        rule: &ScheduleRule,
        current_day: Weekday,
        current_time: chrono::NaiveTime,
    ) -> bool {
        // Check if current day is in the rule's days
        let day_matches = rule.days.iter().any(|d| {
            let weekday: Weekday = (*d).into();
            weekday == current_day
        });

        if !day_matches {
            return false;
        }

        // Check if current time is within the rule's time range
        let start = rule.start_time.0;
        let end = rule.end_time.0;

        // Handle overnight rules (e.g., 22:00 - 06:00)
        if start <= end {
            // Normal range (e.g., 09:00 - 17:00)
            current_time >= start && current_time < end
        } else {
            // Overnight range (e.g., 22:00 - 06:00)
            current_time >= start || current_time < end
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use blockandfocus_shared::NaiveTimeWrapper;
    use chrono::NaiveTime;

    fn make_rule(name: &str, days: Vec<WeekdayWrapper>, start: &str, end: &str) -> ScheduleRule {
        ScheduleRule {
            name: name.to_string(),
            days,
            start_time: NaiveTimeWrapper(NaiveTime::parse_from_str(start, "%H:%M").unwrap()),
            end_time: NaiveTimeWrapper(NaiveTime::parse_from_str(end, "%H:%M").unwrap()),
        }
    }

    #[test]
    fn test_schedule_disabled() {
        let schedule = Schedule {
            enabled: false,
            rules: vec![],
        };
        let engine = ScheduleEngine::new(schedule);

        // When schedule is disabled, blocking is always active
        assert!(engine.is_blocking_time());
    }

    #[test]
    fn test_no_rules() {
        let schedule = Schedule {
            enabled: true,
            rules: vec![],
        };
        let engine = ScheduleEngine::new(schedule);

        // With no rules, blocking is never scheduled
        assert!(!engine.is_blocking_time());
    }

    #[test]
    fn test_time_range_matching() {
        let rule = make_rule(
            "Work Hours",
            vec![
                WeekdayWrapper::Mon,
                WeekdayWrapper::Tue,
                WeekdayWrapper::Wed,
                WeekdayWrapper::Thu,
                WeekdayWrapper::Fri,
            ],
            "09:00",
            "17:00",
        );

        let engine = ScheduleEngine::new(Schedule {
            enabled: true,
            rules: vec![rule.clone()],
        });

        // Test at 10:00 on Monday
        let monday_10am = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        assert!(engine.rule_matches(&rule, Weekday::Mon, monday_10am));

        // Test at 08:00 on Monday (before schedule)
        let monday_8am = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        assert!(!engine.rule_matches(&rule, Weekday::Mon, monday_8am));

        // Test at 10:00 on Saturday (wrong day)
        let saturday_10am = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        assert!(!engine.rule_matches(&rule, Weekday::Sat, saturday_10am));
    }

    #[test]
    fn test_overnight_rule() {
        let rule = make_rule(
            "Night Block",
            vec![
                WeekdayWrapper::Mon,
                WeekdayWrapper::Tue,
                WeekdayWrapper::Wed,
                WeekdayWrapper::Thu,
                WeekdayWrapper::Fri,
            ],
            "22:00",
            "06:00",
        );

        let engine = ScheduleEngine::new(Schedule {
            enabled: true,
            rules: vec![rule.clone()],
        });

        // Test at 23:00 (should match)
        let late_night = NaiveTime::from_hms_opt(23, 0, 0).unwrap();
        assert!(engine.rule_matches(&rule, Weekday::Mon, late_night));

        // Test at 03:00 (should match)
        let early_morning = NaiveTime::from_hms_opt(3, 0, 0).unwrap();
        assert!(engine.rule_matches(&rule, Weekday::Mon, early_morning));

        // Test at 12:00 (should not match)
        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();
        assert!(!engine.rule_matches(&rule, Weekday::Mon, noon));
    }
}
