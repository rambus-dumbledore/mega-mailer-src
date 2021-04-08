use redis::{ErrorKind, FromRedisValue, RedisError, RedisResult, RedisWrite, ToRedisArgs, Value};
use serde::{Deserialize, Serialize};
use serde_cbor;

#[derive(Serialize, Deserialize, Debug)]
pub struct TelegramMessageTask {
    pub to: String,
    pub text: String,
}

impl ToRedisArgs for TelegramMessageTask {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + RedisWrite,
    {
        let encoded = serde_cbor::to_vec(self).unwrap();
        ToRedisArgs::write_redis_args(&encoded, out);
    }
}

impl FromRedisValue for TelegramMessageTask {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        match v {
            Value::Data(data) => {
                let task =
                    serde_cbor::from_slice::<TelegramMessageTask>(data.as_ref()).map_err(|e| {
                        RedisError::from((
                            ErrorKind::TypeError,
                            "Could not deserialize TelegramMessageTask struct",
                        ))
                    })?;
                return Ok(task);
            }
            _ => {}
        }
        Err(RedisError::from((
            ErrorKind::TypeError,
            "Could not deserialize TelegramMessageTask struct",
        )))
    }
}
