use crate::automation::spec::WaitStep;
use crate::telegram::MessageFilter;

pub fn build_wait_filter(step: &WaitStep) -> MessageFilter {
    MessageFilter {
        text_equals: step.text.clone(),
        text_contains: step.text_contains.clone(),
        ..MessageFilter::default()
    }
}
