use rand::distributions::Uniform;
use rand::{thread_rng, Rng};

pub struct Choice {
    pub question_id: String,
    pub answer_state: Option<String>,
    pub answer_answered_at: Option<String>,
    // level: 1, 2, 4, 8, 16, ...,
    // last_seen: Duration,
}

impl Choice {
    #[allow(dead_code)]
    pub fn new(question_id: &str, state: &str) -> Self {
        Self {
            question_id: question_id.to_string(),
            answer_state: Some(state.to_string()),
            answer_answered_at: None,
        }
    }

    fn clone(&self) -> Self {
        Self {
            answer_answered_at: self.answer_answered_at.clone(),
            question_id: self.question_id.clone(),
            answer_state: self.answer_state.clone(),
        }
    }

    // fn last_seen(&self) -> Duration {
    // }
}

pub trait Strategy {
    fn take(&self, n: u64) -> Vec<Choice>;
}

pub struct Random {
    choices: Vec<Choice>,
}

impl Random {
    pub fn new(choices: Vec<Choice>) -> Self {
        Self { choices }
    }
}

impl Strategy for Random {
    fn take(&self, n: u64) -> Vec<Choice> {
        let range = Uniform::new_inclusive(0, self.choices.len() - 1);
        let mut rng = thread_rng();
        let mut gen = (&mut rng).sample_iter(&range);
        let mut selected = Vec::new();
        for _ in 0..n {
            let j = gen.next().unwrap();
            selected.push(self.choices[j].clone());
        }
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_choice() {
        let chooser = Random::new(vec![
            Choice::new("1", "unseen"),
            Choice::new("2", "unseen"),
            Choice::new("3", "unseen"),
        ]);
        assert_eq!(1, chooser.take(1).len());
    }
}
