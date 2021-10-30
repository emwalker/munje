use anyhow::{bail, Error, Result};
use chrono;
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use std::{cmp::Reverse, fmt, iter::FromIterator};

use crate::types::DateTime;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Correct,
    Incorrect,
    Unknown,
    Unseen,
}

pub enum TimeUnit {
    #[allow(dead_code)]
    Days,
    Minutes,
}

pub struct ChoiceRow {
    pub answer_answered_at: Option<String>,
    pub answer_stage: Option<i64>,
    pub answer_state: Option<String>,
    pub question_id: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Choice {
    pub answered_at: DateTime,
    pub question_id: String,
    pub stage: i64,
    pub state: State,
}

pub trait Strategy {
    fn to_vec(&self) -> Vec<Choice>;

    fn filter_choices(&self, choices: &Vec<Choice>) -> Vec<Choice>;

    fn available_at(&self, choice: &Choice) -> DateTime;

    fn next_question(&self) -> Result<(Option<Choice>, DateTime), Error> {
        let total = &self.to_vec();
        let available = self.filter_choices(&total);
        info!(
            "Choosing from {} total and {} available choices",
            total.len(),
            available.len()
        );
        if available.len() > 0 {
            let choice = available[0].clone();
            Ok((Some(choice.clone()), self.available_at(&choice)))
        } else if total.len() > 0 {
            debug!(
                "No choices currently available, choosing first from total choices: {:?}",
                total
            );
            let next_choice = total[0].clone();
            Ok((None, self.available_at(&next_choice)))
        } else {
            // Hopefully we never get here
            bail!("Expected one or more choices")
        }
    }
}

#[derive(Copy, Clone)]
pub struct Clock {
    now: DateTime,
}

pub struct Random {
    choices: Vec<Choice>,
}

/// Questions are produced in a queue according to the following rules:
///
/// 1. If a question has been asked in the past and is ready to show to the user, the question
///    is shown.
///
/// 2. If a question has not been asked in the past and is ready to show, and there are no questions
///    to ask that have already been attempted in the past, the new question is shown to the user
///    for the first time.
///
///  3. A question is ready to show if the number of days since it was last attempted is equal to
///    the stage tracked for the user's progress with that question.
///
/// 4. If a question is attempted and answered incorrectly, the stage for the question is
///    decremented, e.g., from 64 to 32.  The date the question was attempted is noted.  The amount
///    by which the stage is decreased is an implementation detail to be optimized.
///
/// 5. If a question is attempted and answered correctly, the stage for the question is incremented,
///    e.g., from 64 to 128.  The date the question was attempted is noted.  The amount by which
///    the stage is increased is an implementation detail to be optimized.
///
/// In a simple implementation, these rules will be most of what happens.  In a more sophisticated
/// implementation, additional considerations will come to bear on which questions to introduce
/// next and when to stop showing questions to the user.
///
pub struct SpacedRepetition {
    choices: Vec<Choice>,
    clock: Clock,
    unit: TimeUnit,
}

impl Clock {
    fn new() -> Self {
        Self {
            now: DateTime::now(),
        }
    }

    #[allow(dead_code)]
    fn days(&self, n: i64) -> DateTime {
        self.now + chrono::Duration::days(n)
    }

    fn threshold(&self) -> DateTime {
        self.now
    }
}

impl State {
    fn from(maybe_state: Option<String>) -> Self {
        match maybe_state {
            Some(state) => match state.as_ref() {
                "unseen" => Self::Unseen,
                "correct" => Self::Correct,
                "incorrect" => Self::Incorrect,
                _ => Self::Unknown,
            },
            None => Self::Unseen,
        }
    }
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string = match self {
            Self::Unseen => "unseen",
            Self::Correct => "correct",
            Self::Incorrect => "incorrect",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", string)
    }
}

impl ChoiceRow {
    fn to_choice(&self) -> Choice {
        let state = State::from(self.answer_state.clone());
        let answered_at: chrono::DateTime<chrono::Utc> = match &self.answer_answered_at {
            Some(string) => chrono::DateTime::parse_from_rfc3339(string)
                .map(|dt| chrono::DateTime::from(dt))
                .unwrap_or(chrono::Utc::now()),
            None => chrono::Utc::now(),
        };
        let stage = self.answer_stage.unwrap_or(0);
        Choice::new(&self.question_id, stage, DateTime(answered_at), state)
    }
}

impl fmt::Debug for ChoiceRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ChoiceRow {{ {} {} {} }}",
            self.question_id,
            self.answer_stage.unwrap_or(-1),
            self.answer_state.clone().unwrap_or("None".to_string()),
        )
    }
}

impl Choice {
    #[allow(dead_code)]
    pub fn new(question_id: &str, stage: i64, answered_at: DateTime, state: State) -> Self {
        Self {
            answered_at: answered_at,
            question_id: question_id.to_string(),
            stage,
            state: state,
        }
    }

    fn clone(&self) -> Self {
        Self {
            answered_at: self.answered_at.clone(),
            question_id: self.question_id.clone(),
            stage: self.stage,
            state: self.state.clone(),
        }
    }
}

impl<'c> FromIterator<&'c Choice> for Vec<Choice> {
    fn from_iter<I: IntoIterator<Item = &'c Choice>>(iter: I) -> Self {
        let mut choices = Vec::new();
        for c in iter {
            choices.push(c.clone());
        }
        choices
    }
}

#[allow(dead_code)]
impl Random {
    pub fn from_rows(choices: Vec<ChoiceRow>) -> Self {
        Self::new(choices.iter().map(ChoiceRow::to_choice).collect())
    }

    fn new(choices: Vec<Choice>) -> Self {
        Self { choices }
    }
}

impl Strategy for Random {
    // FIXME: Re-implement as a generator
    fn to_vec(&self) -> Vec<Choice> {
        let n = self.choices.len();
        debug!("Selecting choices from range 0 - {}", n - 1);
        let range = Uniform::new_inclusive(0, n - 1);
        let mut rng = thread_rng();
        let mut gen = (&mut rng).sample_iter(&range);
        let mut selected = Vec::with_capacity(n);
        for _ in 0..n {
            let j = gen.next().unwrap();
            selected.push(self.choices[j].clone());
        }
        selected
    }

    fn available_at(&self, _choice: &Choice) -> DateTime {
        DateTime::now()
    }

    fn filter_choices(&self, choices: &Vec<Choice>) -> Vec<Choice> {
        choices.clone()
    }
}

impl SpacedRepetition {
    pub fn from_rows(choices: Vec<ChoiceRow>, unit: TimeUnit) -> Self {
        Self::new(
            choices.iter().map(ChoiceRow::to_choice).collect(),
            Clock::new(),
            unit,
        )
    }

    pub fn new(choices: Vec<Choice>, clock: Clock, unit: TimeUnit) -> Self {
        Self {
            choices,
            clock,
            unit,
        }
    }

    fn available_at(&self, choice: &Choice) -> DateTime {
        let delta = match self.unit {
            TimeUnit::Days => chrono::Duration::days(choice.stage),
            TimeUnit::Minutes => chrono::Duration::minutes(choice.stage),
        };
        choice.answered_at.clone() + delta
    }

    fn available(&self, choice: &Choice) -> bool {
        self.available_at(choice) <= self.clock.threshold()
    }
}

impl Strategy for SpacedRepetition {
    // A timestamp is computed by taking the date at which the question was last attempted and then
    // adding stage number of days to that timestamp.  Once the new timestamps are obtained, the
    // questions are ordered in ascending order, so that choices with timestamps closest to today's
    // date appear first.
    fn to_vec(&self) -> Vec<Choice> {
        let mut choices = self.choices.clone();
        let threshold = self.clock.threshold();
        choices.sort_by_key(|c| (c.question_id.clone(), Reverse(c.answered_at)));
        choices.dedup_by_key(|c| c.question_id.clone());
        choices.sort_by_key(|c| (c.stage, Reverse(threshold - c.answered_at)));
        choices
    }

    fn filter_choices(&self, choices: &Vec<Choice>) -> Vec<Choice> {
        choices
            .iter()
            .filter(|c| self.available(c))
            .collect::<Vec<Choice>>()
    }

    fn available_at(&self, choice: &Choice) -> DateTime {
        let delta = match self.unit {
            TimeUnit::Minutes => chrono::Duration::minutes(choice.stage),
            TimeUnit::Days => chrono::Duration::days(choice.stage),
        };
        choice.answered_at + delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(non_snake_case)]
    fn C(id: &str, stage: i64, days: DateTime, state: State) -> Choice {
        Choice::new(id, stage, days, state)
    }

    #[test]
    fn random_choice() {
        let clock = Clock::new();
        let chooser = Random::new(vec![
            Choice::new("1", 0, clock.days(0), State::Unseen),
            Choice::new("2", 0, clock.days(0), State::Correct),
            Choice::new("3", 0, clock.days(0), State::Incorrect),
        ]);
        assert_eq!(1, chooser.to_vec().iter().take(1).len());
    }

    #[test]
    fn next_question() {
        let clock = Clock::new();
        let chooser = Random::new(vec![
            Choice::new("1", 0, clock.days(0), State::Unseen),
            Choice::new("2", 0, clock.days(0), State::Correct),
            Choice::new("3", 0, clock.days(0), State::Incorrect),
        ]);
        let (question, _) = chooser.next_question().unwrap();
        assert_ne!(None, question);
    }

    #[test]
    fn spaced_repetition() {
        let clock = Clock::new();

        struct TestCase<'s> {
            name: &'s str,
            choices: Vec<Choice>,
            expected: (Option<usize>, DateTime),
        }

        let cases = [
            TestCase {
                name: "A simple case",
                choices: vec![
                    C("0", 1, clock.days(1), State::Unseen),
                    C("1", 4, clock.days(-1), State::Correct),
                    C("2", 1, clock.days(-2), State::Incorrect),
                ],
                expected: (Some(2), clock.days(-1)),
            },
            TestCase {
                name: "When a question is not ready to work on yet",
                choices: vec![C("1", 2, clock.days(0), State::Correct)],
                expected: (None, clock.days(2)),
            },
            TestCase {
                name: "When there are several questions that are ready to work on",
                choices: vec![
                    C("0", 2, clock.days(-2), State::Correct),
                    C("1", 4, clock.days(-3), State::Correct),
                    C("2", 8, clock.days(-10), State::Correct),
                ],
                expected: (Some(0), clock.days(0)),
            },
            TestCase {
                name: "When there are several questions, none of which is ready to work on",
                choices: vec![
                    C("0", 2, clock.days(0), State::Correct),
                    C("1", 4, clock.days(0), State::Correct),
                    C("2", 8, clock.days(0), State::Correct),
                ],
                expected: (None, clock.days(2)),
            },
            TestCase {
                name: "When there is more than one choice for the same question",
                choices: vec![
                    C("0", 2, clock.days(0), State::Correct),
                    C("0", 1, clock.days(-1), State::Incorrect),
                    C("0", 1, clock.days(-2), State::Incorrect),
                ],
                expected: (None, clock.days(2)),
            },
            TestCase {
                name: "Another case where no questions are ready",
                choices: vec![
                    C("0", 2, clock.days(0), State::Correct),
                    C("1", 2, clock.days(-1), State::Correct),
                    C("2", 16, clock.days(-1), State::Correct),
                ],
                expected: (None, clock.days(1)),
            },
            TestCase {
                name: "A third case where no questions are ready",
                choices: vec![
                    C("0", 2, clock.days(0), State::Correct),
                    C("1", 2, clock.days(-1), State::Correct),
                    C("2", 16, clock.days(-1), State::Correct),
                ],
                expected: (None, clock.days(1)),
            },
        ];

        for case in cases {
            let chooser =
                SpacedRepetition::new(case.choices.clone(), clock.clone(), TimeUnit::Days);

            let (choice, available_at) = chooser.next_question().unwrap();
            let expected_choice = match case.expected.0 {
                Some(index) => Some(case.choices[index].clone()),
                None => None,
            };

            assert_eq!(expected_choice, choice, "{}", case.name);
            assert_eq!(case.expected.1, available_at, "{}", case.name);
        }
    }
}
