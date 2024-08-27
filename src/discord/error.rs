use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ModelConfigurationBuilder(#[from] chatgpt::config::ModelConfigurationBuilderError),
    #[error(transparent)]
    ChatGPT(#[from] chatgpt::err::Error),
    #[error(transparent)]
    Client(#[from] serenity::Error),
}
