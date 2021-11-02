use anyhow::{bail, Error, Result};
use chrono;
use rand::distributions::Uniform;
use rand::{thread_rng, Rng};
use std::{cmp::Reverse, convert::TryFrom, fmt, iter::FromIterator};

use crate::types::DateTime;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Correct,
    Incorrect,
    Unknown,
    Unseen,
}

#[derive(Copy, Clone)]
pub enum TimeUnit {
    #[allow(dead_code)]
    Days,
    Minutes,
}

// The answer-related fields are Option, because they are taken from a left join against the
// questions table.actix_http
pub struct ChoiceRow {
    pub answer_answered_at: Option<chrono::DateTime<chrono::Utc>>,
    pub answer_consecutive_correct: Option<i32>,
    pub answer_state: Option<String>,
    pub question_id: i64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Choice {
    stage: i32,
    pub answered_at: DateTime,
    pub consecutive_correct: i32,
    pub question_id: i64,
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
    unit: TimeUnit,
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
    fn new(unit: TimeUnit) -> Self {
        Self {
            now: DateTime::now(),
            unit,
        }
    }

    #[allow(dead_code)]
    fn ticks(&self, n: i64) -> DateTime {
        self.now
            + match self.unit {
                TimeUnit::Days => chrono::Duration::days(n),
                TimeUnit::Minutes => chrono::Duration::minutes(n),
            }
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
            None => Self::Unknown,
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
    fn to_choice(&self, clock: &Clock) -> Choice {
        let state = State::from(self.answer_state.clone());
        let already = clock.ticks(-2).to_chrono();
        let answered_at = self.answer_answered_at.unwrap_or(already);
        let consecutive_correct = self.answer_consecutive_correct.unwrap_or(0);

        Choice::new(
            self.question_id,
            DateTime(answered_at),
            consecutive_correct,
            state,
        )
    }
}

impl fmt::Debug for ChoiceRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "ChoiceRow {{ {} {} {} }}",
            self.question_id,
            self.answer_consecutive_correct.unwrap_or(0),
            self.answer_state.clone().unwrap_or("unknown".to_string()),
        )
    }
}

impl Choice {
    #[allow(dead_code)]
    pub fn new(
        question_id: i64,
        answered_at: DateTime,
        consecutive_correct: i32,
        state: State,
    ) -> Self {
        Self {
            answered_at,
            consecutive_correct,
            question_id,
            stage: Self::stage_from(consecutive_correct),
            state,
        }
    }

    pub fn stage_from(consecutive_correct: i32) -> i32 {
        let base: i32 = 2;
        base.pow(u32::try_from(consecutive_correct).unwrap_or(0))
    }

    fn clone(&self) -> Self {
        Self {
            answered_at: self.answered_at.clone(),
            consecutive_correct: self.consecutive_correct,
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
        let clock = Clock::new(TimeUnit::Days);
        Self::new(choices.iter().map(|row| row.to_choice(&clock)).collect())
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
        let clock = Clock::new(unit);
        Self::new(
            choices.iter().map(|row| row.to_choice(&clock)).collect(),
            clock,
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
            TimeUnit::Days => chrono::Duration::days(choice.stage.into()),
            TimeUnit::Minutes => chrono::Duration::minutes(choice.stage.into()),
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
        choices.sort_by_key(|c| (c.consecutive_correct, Reverse(threshold - c.answered_at)));
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
            TimeUnit::Minutes => chrono::Duration::minutes(choice.stage.into()),
            TimeUnit::Days => chrono::Duration::days(choice.stage.into()),
        };
        choice.answered_at + delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(non_snake_case)]
    fn C(id: i64, consecutive_correct: i32, answered_at: DateTime, state: State) -> Choice {
        Choice::new(id, answered_at, consecutive_correct, state)
    }

    #[test]
    fn random_choice() {
        let clock = Clock::new(TimeUnit::Days);
        let chooser = Random::new(vec![
            C(1, 0, clock.ticks(0), State::Unseen),
            C(2, 0, clock.ticks(0), State::Correct),
            C(3, 0, clock.ticks(0), State::Incorrect),
        ]);
        assert_eq!(1, chooser.to_vec().iter().take(1).len());
    }

    #[test]
    fn next_question() {
        let clock = Clock::new(TimeUnit::Days);
        let chooser = Random::new(vec![
            C(1, 0, clock.ticks(0), State::Unseen),
            C(2, 0, clock.ticks(0), State::Correct),
            C(3, 0, clock.ticks(0), State::Incorrect),
        ]);
        let (question, _) = chooser.next_question().unwrap();
        assert_ne!(None, question);
    }

    #[test]
    fn spaced_repetition() {
        let clock = Clock::new(TimeUnit::Minutes);

        struct TestCase<'s> {
            name: &'s str,
            choices: Vec<Choice>,
            expected: (Option<usize>, DateTime),
        }

        let cases = [
            TestCase {
                name: "A simple case",
                choices: vec![
                    C(0, 0, clock.ticks(1), State::Unseen),
                    C(1, 2, clock.ticks(-1), State::Correct),
                    C(2, 0, clock.ticks(-2), State::Incorrect),
                ],
                expected: (Some(2), clock.ticks(-1)),
            },
            TestCase {
                name: "When a question is not ready to work on yet",
                choices: vec![C(1, 2, clock.ticks(0), State::Correct)],
                expected: (None, clock.ticks(4)),
            },
            TestCase {
                name: "When there are several questions that are ready to work on",
                choices: vec![
                    C(0, 1, clock.ticks(-2), State::Correct),
                    C(1, 2, clock.ticks(-3), State::Correct),
                    C(2, 3, clock.ticks(-10), State::Correct),
                ],
                expected: (Some(0), clock.ticks(0)),
            },
            TestCase {
                name: "When there are several questions, none of which is ready to work on",
                choices: vec![
                    C(0, 1, clock.ticks(0), State::Correct),
                    C(1, 2, clock.ticks(0), State::Correct),
                    C(2, 3, clock.ticks(0), State::Correct),
                ],
                expected: (None, clock.ticks(2)),
            },
            TestCase {
                name: "When there is more than one choice for the same question",
                choices: vec![
                    C(0, 1, clock.ticks(0), State::Correct),
                    C(0, 0, clock.ticks(-1), State::Incorrect),
                    C(0, 0, clock.ticks(-2), State::Incorrect),
                ],
                expected: (None, clock.ticks(2)),
            },
            TestCase {
                name: "Another case where no questions are ready",
                choices: vec![
                    C(0, 1, clock.ticks(0), State::Correct),
                    C(1, 1, clock.ticks(-1), State::Correct),
                    C(2, 4, clock.ticks(-1), State::Correct),
                ],
                expected: (None, clock.ticks(1)),
            },
            TestCase {
                name: "A third case where no questions are ready",
                choices: vec![
                    C(0, 1, clock.ticks(0), State::Correct),
                    C(1, 1, clock.ticks(-1), State::Correct),
                    C(2, 4, clock.ticks(-1), State::Correct),
                ],
                expected: (None, clock.ticks(1)),
            },
            TestCase {
                name: "When there are no answers in the queue yet",
                choices: vec![
                    ChoiceRow {
                        question_id: 0,
                        answer_answered_at: None,
                        answer_consecutive_correct: None,
                        answer_state: None,
                    }
                    .to_choice(&clock),
                    ChoiceRow {
                        question_id: 1,
                        answer_answered_at: None,
                        answer_consecutive_correct: None,
                        answer_state: None,
                    }
                    .to_choice(&clock),
                ],
                expected: (Some(0), clock.ticks(-1)),
            },
        ];

        for case in cases {
            let chooser =
                SpacedRepetition::new(case.choices.clone(), clock.clone(), TimeUnit::Minutes);

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
