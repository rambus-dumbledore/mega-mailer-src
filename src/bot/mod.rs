mod handlers;

use tokio;
use std::sync::Once;
use teloxide::{prelude::*, utils::command::BotCommand};
use std::sync::Arc;
use teloxide::types::ParseMode::{MarkdownV2};

use crate::cfg::CONFIG;
use crate::storage::Storage;
use crate::types::Error;

#[derive(Clone)]
pub struct TelegramBot {
    bot: Bot,
    storage: Arc<Storage>,
}

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "attach telegram account to user account")]
    Attach(String),
    #[command(description = "set account avatar from telegram")]
    SetAvatar
}

impl TelegramBot {
    pub fn new(storage: Arc<Storage>) -> TelegramBot {
        let token = CONFIG.get::<String>("bot.secret");
        let b = Bot::new(token);

        let bot = TelegramBot{
            bot: b,
            storage
        };

        let b = bot.bot.clone();
        let s = bot.storage.clone();

        static INIT_LISTENER: Once = Once::new();
        INIT_LISTENER.call_once(|| {
            tokio::spawn(async move {
                teloxide::commands_repl(b, "MegaMailerBot", move |cx, command| {
                    TelegramBot::answer(cx, command, Arc::clone(&s))
                }).await;
            });
        });

        bot
    }

    async fn answer(cx: UpdateWithCx<Bot, Message>, command: Command, storage: Arc<Storage>) -> Result<(), Error> {
        match command {
            Command::Help => {
                cx.answer(Command::descriptions()).send();
            },
            Command::Attach(code) => { handlers::process_attach_command(cx, &storage, &code).await? },
            Command::SetAvatar => { handlers::process_set_avatar_command(cx, &storage).await? }
        };

        Ok(())
    }

    pub async fn send_login_code(&self, chat_id: String, username: &String) -> Result<(), Error> {
        let code = self.storage.create_login_request(username);
        self.bot.send_message(chat_id, format!("Your login code: {}", code)).send().await?;
        Ok(())
    }

    pub async fn send_markdown(&self, username: &String, text: &String) -> Result<(), Error> {
        let chat_id = self.storage.get_telegram_id(username)?;
        self.bot.send_message(chat_id, text).parse_mode(MarkdownV2).send().await?;
        Ok(())
    }


}
