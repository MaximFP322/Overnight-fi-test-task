#[derive(Debug)]
pub enum EngineActorError<E> {
    Processor(E),
    SequenceExhausted,
}
