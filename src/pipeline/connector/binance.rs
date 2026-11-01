use crate::client::{BlockingHttpClient, BlockingHttpClientError};
use crate::clock::{Clock, SystemClock};
use crate::connector::binance::{BinanceDiffWsClient, BinanceMarketProcessor, BinanceRest};
use crate::pipeline::{ActorKind, ConnectorPorts, LivePipelineError, Pipeline, PipelineConfig, PipelineError};

pub struct BinanceConnector;

impl BinanceConnector {
    pub fn run(config: &PipelineConfig, ports: ConnectorPorts, pipeline: &mut Pipeline) -> Result<(), LivePipelineError> {
        let processor = Self::create(config, SystemClock).map_err(|source| PipelineError::ConnectorCreate {
            actor: ActorKind::BinanceConnector,
            source,
        })?;
        crate::pipeline::runtime::start_binance(processor, ports, pipeline)
    }

    pub fn create<CLOCK: Clock + Clone>(
        config: &PipelineConfig,
        clock: CLOCK,
    ) -> Result<BinanceMarketProcessor<BinanceDiffWsClient, BinanceRest<CLOCK>, CLOCK>, BlockingHttpClientError> {
        let http = BlockingHttpClient::new(config.http_client.clone())?;
        let ws = BinanceDiffWsClient::new(&config.binance);
        let rest = BinanceRest::new(http, clock.clone(), config.rest_retry.clone(), &config.binance);
        Ok(BinanceMarketProcessor::new(config.binance.clone(), ws, rest, clock))
    }
}
