use super::*;

#[cfg(test)]
mod tests;

use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnonymizeUpdateOp {
    /// Replaces every student's and teacher's name with a fake one and drops
    /// their contact details.
    ///
    /// The seed is the op's whole payload: applying it is a pure function of
    /// the seed and the document it lands on, so the same op replayed on the
    /// same document gives the same names back.
    AnonymizeNames { seed: u64 },
}

#[derive(Clone, Debug, Error, Serialize, Deserialize, PartialEq, Eq)]
pub enum AnonymizeUpdateError {}

const FIRSTNAMES: [&str; 64] = [
    "Adrien",
    "Alice",
    "Amélie",
    "Antoine",
    "Arthur",
    "Aurélie",
    "Baptiste",
    "Benjamin",
    "Camille",
    "Céline",
    "Charlotte",
    "Chloé",
    "Clara",
    "Clément",
    "Damien",
    "David",
    "Élise",
    "Émilie",
    "Emma",
    "Enzo",
    "Étienne",
    "Fabien",
    "Florence",
    "Gabriel",
    "Guillaume",
    "Hugo",
    "Inès",
    "Isabelle",
    "Jeanne",
    "Jérôme",
    "Julien",
    "Juliette",
    "Justine",
    "Laura",
    "Laurent",
    "Léa",
    "Léo",
    "Louis",
    "Lucas",
    "Lucie",
    "Manon",
    "Marie",
    "Mathieu",
    "Mathilde",
    "Maxime",
    "Mélanie",
    "Nathalie",
    "Nicolas",
    "Noémie",
    "Olivier",
    "Pauline",
    "Pierre",
    "Quentin",
    "Raphaël",
    "Rémi",
    "Romain",
    "Sarah",
    "Sébastien",
    "Sophie",
    "Théo",
    "Thomas",
    "Valentin",
    "Vincent",
    "Zoé",
];

const SURNAMES: [&str; 64] = [
    "Martin",
    "Bernard",
    "Dubois",
    "Durand",
    "Moreau",
    "Laurent",
    "Simon",
    "Michel",
    "Lefebvre",
    "Leroy",
    "Roux",
    "David",
    "Bertrand",
    "Morel",
    "Fournier",
    "Girard",
    "Bonnet",
    "Dupont",
    "Lambert",
    "Fontaine",
    "Rousseau",
    "Vincent",
    "Muller",
    "Lefèvre",
    "Faure",
    "André",
    "Mercier",
    "Blanc",
    "Guérin",
    "Boyer",
    "Garnier",
    "Chevalier",
    "François",
    "Legrand",
    "Gauthier",
    "Garcia",
    "Perrin",
    "Robin",
    "Clément",
    "Morin",
    "Nicolas",
    "Henry",
    "Roussel",
    "Mathieu",
    "Gautier",
    "Masson",
    "Marchand",
    "Duval",
    "Denis",
    "Dumont",
    "Marie",
    "Lemaire",
    "Noël",
    "Meyer",
    "Dufour",
    "Meunier",
    "Brun",
    "Blanchard",
    "Giraud",
    "Joly",
    "Rivière",
    "Lucas",
    "Brunet",
    "Gaillard",
];

/// Hands out fake names, never twice the same one.
///
/// Uniqueness without a draw-and-retry loop: the sampler shuffles the whole
/// (firstname, surname) product once and then walks it in order. Past the
/// 4096th person — which no colloscope reaches — the walk comes round again and
/// a numeric suffix on the surname keeps the second lap apart from the first.
struct NameSampler {
    /// The pair indices, shuffled once.
    order: Vec<usize>,
    /// How many names have been handed out.
    handed_out: usize,
}

impl NameSampler {
    fn new(seed: u64) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut order: Vec<usize> = (0..FIRSTNAMES.len() * SURNAMES.len()).collect();
        order.shuffle(&mut rng);

        NameSampler {
            order,
            handed_out: 0,
        }
    }

    /// The next (firstname, surname) pair.
    fn next_name(&mut self) -> (String, String) {
        let lap = self.handed_out / self.order.len();
        let pair = self.order[self.handed_out % self.order.len()];
        self.handed_out += 1;

        let firstname = FIRSTNAMES[pair / SURNAMES.len()].to_string();
        let surname = SURNAMES[pair % SURNAMES.len()];
        let surname = if lap == 0 {
            surname.to_string()
        } else {
            format!("{surname} {}", lap + 1)
        };

        (firstname, surname)
    }
}

impl AnonymizeUpdateOp {
    pub(crate) fn apply_to_session<
        T: collomatique_state::traits::Manager<Data = Data, Desc = Desc>,
    >(
        &self,
        session: &mut CascadeSession<T>,
    ) -> Result<(), AnonymizeUpdateError> {
        let Self::AnonymizeNames { seed } = self;

        let mut sampler = NameSampler::new(*seed);

        // Students first, then teachers, both in id order: the sampler's output
        // depends on how many names it has already handed out, so the walk order
        // is part of what makes the op a function of its seed.
        let student_map = session
            .get_data()
            .get_inner_data()
            .params
            .students
            .student_map
            .clone();
        for (student_id, student) in student_map.iter() {
            let (firstname, surname) = sampler.next_name();
            let mut new_student = student.clone();
            new_student.desc = collomatique_state_colloscopes::PersonWithContact {
                surname,
                firstname,
                tel: None,
                email: None,
            };

            let result = session
                .apply(
                    collomatique_state_colloscopes::Op::Student(
                        collomatique_state_colloscopes::StudentOp::Update(student_id, new_student),
                    ),
                    self.get_desc(),
                )
                // `desc` is the one field of a student no foreign key and no
                // convergence predicate reads: the document cannot notice the
                // rename, so there is nothing for it to reject or repair.
                .expect("renaming a live student contradicts nothing");
            assert!(result.is_none());
        }

        let teacher_map = session
            .get_data()
            .get_inner_data()
            .params
            .teachers
            .teacher_map
            .clone();
        for (teacher_id, teacher) in teacher_map.iter() {
            let (firstname, surname) = sampler.next_name();
            let mut new_teacher = teacher.clone();
            new_teacher.desc = collomatique_state_colloscopes::PersonWithContact {
                surname,
                firstname,
                tel: None,
                email: None,
            };

            let result = session
                .apply(
                    collomatique_state_colloscopes::Op::Teacher(
                        collomatique_state_colloscopes::TeacherOp::Update(teacher_id, new_teacher),
                    ),
                    self.get_desc(),
                )
                // Same argument as the students: only `desc` moves.
                .expect("renaming a live teacher contradicts nothing");
            assert!(result.is_none());
        }

        Ok(())
    }

    pub fn get_desc(&self) -> (OpCategory, String) {
        (
            OpCategory::Anonymize,
            match self {
                AnonymizeUpdateOp::AnonymizeNames { .. } => "Anonymiser les noms".into(),
            },
        )
    }
}
