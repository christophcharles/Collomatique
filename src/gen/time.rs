#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Weekday {
    #[default]
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub fn iter() -> impl Iterator<Item = Self> {
        [
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
        ]
        .iter()
        .copied()
    }
}

impl std::fmt::Display for Weekday {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                Weekday::Monday => "Lundi",
                Weekday::Tuesday => "Mardi",
                Weekday::Wednesday => "Mercredi",
                Weekday::Thursday => "Jeudi",
                Weekday::Friday => "Vendredi",
                Weekday::Saturday => "Samedi",
                Weekday::Sunday => "Dimanche",
            },
        )
    }
}

impl From<Weekday> for usize {
    fn from(value: Weekday) -> usize {
        match value {
            Weekday::Monday => 0,
            Weekday::Tuesday => 1,
            Weekday::Wednesday => 2,
            Weekday::Thursday => 3,
            Weekday::Friday => 4,
            Weekday::Saturday => 5,
            Weekday::Sunday => 6,
        }
    }
}

impl From<&Weekday> for usize {
    fn from(value: &Weekday) -> usize {
        (*value).into()
    }
}

impl PartialOrd for Weekday {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Weekday {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        usize::from(*self).cmp(&other.into())
    }
}

impl TryFrom<&str> for Weekday {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Lundi" => Ok(Weekday::Monday),
            "Mardi" => Ok(Weekday::Tuesday),
            "Mercredi" => Ok(Weekday::Wednesday),
            "Jeudi" => Ok(Weekday::Thursday),
            "Vendredi" => Ok(Weekday::Friday),
            "Samedi" => Ok(Weekday::Saturday),
            "Dimanche" => Ok(Weekday::Sunday),
            _ => Err("Unknown weekday"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Time {
    time_in_minutes: u32,
}

const MINUTES_PER_HOUR: u32 = 60;
const HOUR_PER_DAY: u32 = 24;
const MINUTES_PER_DAY: u32 = MINUTES_PER_HOUR * HOUR_PER_DAY;

impl Time {
    pub fn new(min: u32) -> Option<Self> {
        if min >= MINUTES_PER_DAY {
            return None;
        }
        Some(Time {
            time_in_minutes: min,
        })
    }

    pub fn from_hm(hour: u32, min: u32) -> Option<Self> {
        if min >= MINUTES_PER_HOUR {
            return None;
        }
        if hour >= HOUR_PER_DAY {
            return None;
        }

        Some(Time {
            time_in_minutes: hour * MINUTES_PER_HOUR + min,
        })
    }

    pub fn get(&self) -> u32 {
        self.time_in_minutes
    }

    pub fn get_hour(&self) -> u32 {
        self.time_in_minutes / MINUTES_PER_HOUR
    }

    pub fn get_min(&self) -> u32 {
        self.time_in_minutes % MINUTES_PER_HOUR
    }

    pub fn add(&self, duration_in_minutes: u32) -> Option<Self> {
        self.time_in_minutes
            .checked_add(duration_in_minutes)
            .and_then(|x| Self::new(x))
    }
}

impl Default for Time {
    fn default() -> Self {
        Time::from_hm(0, 0).unwrap()
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time_in_minutes.cmp(&other.time_in_minutes)
    }
}
