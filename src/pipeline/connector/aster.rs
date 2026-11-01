use crate::client::{BlockingHttpClient, BlockingHttpClientError};
use crate::clock::{Clock, SystemClock};
use crate::connector::aster::{AsterDiffWsClient, AsterMarketProcessor, AsterRest};
use crate::pipeline::{ActorKind, ConnectorPorts, LivePipelineError, Pipeline, PipelineConfig, PipelineError};

pub struct AsterConnector;

impl AsterConnector {
    pub fn run(config: &PipelineConfig, ports: ConnectorPorts, pipeline: &mut Pipeline) -> Result<(), LivePipelineError> {
        let processor = Self::create(config, SystemClock).map_err(|source| PipelineError::ConnectorCreate {
            actor: ActorKind::AsterConnector,
            source,
        })?;
        crate::pipeline::runtime::start_aster(processor, ports, pipeline)
    }

    pub fn create<CLOCK: Clock + Clone>(
        config: &PipelineConfig,
        clock: CLOCK,
    ) -> Result<AsterMarketProcessor<AsterDiffWsClient, AsterRest<CLOCK>, CLOCK>, BlockingHttpClientError> {
        let http = BlockingHttpClient::new(config.http_client.clone())?;
        let ws = AsterDiffWsClient::new(&config.aster);
        let rest = AsterRest::new(http, clock.clone(), config.rest_retry.clone(), &config.aster);
        Ok(AsterMarketProcessor::new(config.aster.clone(), ws, rest, clock))
    }
}
