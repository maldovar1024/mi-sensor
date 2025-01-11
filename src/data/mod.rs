pub mod html;

use crate::sensor::BYTES_PER_DATUM;
use chrono::{DateTime, Datelike, FixedOffset};
use std::iter::Peekable;

const TIMEZONE: Option<FixedOffset> = FixedOffset::east_opt(8 * 3600);

#[derive(Default)]
pub struct DataItem {
    pub time: DateTime<FixedOffset>,
    pub max_temperature: i16,
    pub min_temperature: i16,
    pub max_humidity: u8,
    pub min_humidity: u8,
}

#[inline(always)]
pub fn get_temp(t: i16) -> String {
    format!("{}.{}", t / 100, t % 100)
}

impl From<&[u8]> for DataItem {
    fn from(value: &[u8]) -> Self {
        assert_eq!(value.len(), BYTES_PER_DATUM);
        DataItem {
            time: DateTime::from_timestamp(
                u32::from_le_bytes(value[0..4].try_into().unwrap()) as i64,
                0,
            )
            .unwrap()
            .with_timezone(&TIMEZONE.unwrap()),
            max_temperature: i16::from_le_bytes(value[4..6].try_into().unwrap()),
            min_temperature: i16::from_le_bytes(value[7..9].try_into().unwrap()),
            max_humidity: value[6],
            min_humidity: value[9],
        }
    }
}

trait Item {
    fn get_item_data(&self) -> &DataItem;
}

impl Item for DataItem {
    #[inline(always)]
    fn get_item_data(&self) -> &DataItem {
        self
    }
}

#[derive(Default)]
pub struct Summary<T> {
    pub summary: DataItem,
    pub details: Box<[T]>,
}

impl<T> Item for Summary<T> {
    #[inline(always)]
    fn get_item_data(&self) -> &DataItem {
        &self.summary
    }
}

struct SummaryIter<S, I: Iterator<Item = S>> {
    iter: Peekable<I>,
}

impl<S, I: Iterator<Item = S>> From<I> for SummaryIter<S, I> {
    fn from(value: I) -> Self {
        Self {
            iter: value.peekable(),
        }
    }
}

macro_rules! summary {
    ($(type $time_range: ident -> $sub: ty, $iter: ident -> $time_fn: ident);+) => {
    $(
        pub type $time_range = Summary<$sub>;

        type $iter<I> = SummaryIter<$sub, I>;

        impl<I: Iterator<Item = $sub>> Iterator for $iter<I> {
            type Item = $time_range;

            fn next(&mut self) -> Option<Self::Item> {
                if let Some(start) = self.iter.next() {
                    let start_time = start.get_item_data().time;
                    let cur_date = start_time.$time_fn();
                    
                    let start_item_data = start.get_item_data();
                    let mut max_temperature = start_item_data.max_temperature;
                    let mut min_temperature = start_item_data.min_temperature;
                    let mut max_humidity = start_item_data.max_humidity;
                    let mut min_humidity = start_item_data.min_humidity;

                    let mut data = vec![start];

                    while self.iter.peek().is_some_and(|item| item.get_item_data().time.$time_fn() == cur_date) {
                        let item = self.iter.next().unwrap();
                        let item_data = item.get_item_data();
                        max_temperature = max_temperature.max(item_data.max_temperature);
                        min_temperature = min_temperature.min(item_data.min_temperature);
                        max_humidity = max_humidity.max(item_data.max_humidity);
                        min_humidity = min_humidity.min(item_data.min_humidity);
                        data.push(item);
                    }

                    return Some(Self::Item {
                        summary: DataItem {
                            time: start_time,
                            max_temperature,
                            min_temperature,
                            max_humidity,
                            min_humidity,
                        },
                        details: data.into_boxed_slice(),
                    });
                }

                None
            }
        }
    )+};
}

summary! {
    type Day -> DataItem, DayIter -> day;
    type Month -> Day, MonthIter -> month;
    type Year -> Month, YearIter -> year
}

pub fn get_summary(data: &[u8]) -> Box<[Year]> {
    assert!(data.len() % BYTES_PER_DATUM == 0);

    YearIter::from(MonthIter::from(DayIter::from(
        data.chunks_exact(BYTES_PER_DATUM).map(DataItem::from),
    )))
    .collect()
}
