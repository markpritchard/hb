use std::sync::atomic::{AtomicUsize, Ordering};

use rand::{thread_rng, Rng};

use crate::config::RequestOrder;

/// Creates an index supplier based on the nominated request order etc
pub(crate) fn create_supplier(
    order: &RequestOrder,
    index_limit: usize,
    num_requests: usize,
) -> Box<dyn IndexSupplier> {
    match order {
        RequestOrder::Sequential => Box::new(SequentialIndex::new(index_limit, num_requests)),
        RequestOrder::Random => Box::new(RandomIndex::new(index_limit, num_requests)),
    }
}

/// Returns the next index into the URL list
pub(crate) trait IndexSupplier: Send {
    // Return the next index or None if no more URLs need to be generated
    fn next_index(&self) -> Option<usize>;
}

// Returns a random index bounded by the number of URLs
struct RandomIndex {
    count: AtomicUsize,
    limit: usize,
    requests: usize,
}

impl RandomIndex {
    pub fn new(limit: usize, requests: usize) -> RandomIndex {
        RandomIndex {
            count: AtomicUsize::new(0),
            limit,
            requests,
        }
    }
}

impl IndexSupplier for RandomIndex {
    fn next_index(&self) -> Option<usize> {
        // Bump the number of URLs we have generated
        let count = self.count.fetch_add(1, Ordering::Relaxed);

        // Extract the next URL if we haven't generated sufficient URLs
        if count < self.requests {
            Some(thread_rng().gen_range(0..self.limit))
        } else {
            None
        }
    }
}

// Iterates through the URLs wrapping back to index=0 if the number of URLs is insufficient to generate the number of requests
struct SequentialIndex {
    next: AtomicUsize,
    limit: usize,
    requests: usize,
}

impl SequentialIndex {
    pub fn new(limit: usize, requests: usize) -> SequentialIndex {
        SequentialIndex {
            next: AtomicUsize::new(0),
            limit,
            requests,
        }
    }
}

impl IndexSupplier for SequentialIndex {
    fn next_index(&self) -> Option<usize> {
        // Fetch the next index in the url list
        let index = self.next.fetch_add(1, Ordering::Relaxed);

        // Extract the next URL if we haven't generated sufficient URLs
        if index < self.requests {
            Some(index % self.limit)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Verifies that the random index generator creates correctly bounded values
    #[test]
    fn random_index() {
        let index_generator = RandomIndex::new(10, 5);
        let mut actual = Vec::new();
        while let Some(i) = index_generator.next_index() {
            actual.push(i);
        }
        assert_eq!(5, actual.len());
        for v in actual {
            assert!(v < 10);
        }
    }

    // Verifies that the sequential generator creates indexes in order
    #[test]
    fn sequential_index() {
        let index_generator = SequentialIndex::new(2, 5);
        let mut actual = Vec::new();
        while let Some(i) = index_generator.next_index() {
            actual.push(i);
        }
        let expected: Vec<usize> = vec![0, 1, 0, 1, 0];
        assert_eq!(expected, actual);
    }
}
