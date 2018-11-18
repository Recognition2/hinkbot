package main

import (
	"github.com/go-telegram-bot-api/telegram-bot-api"
	"sync"
)

type global struct {
	wg        *sync.WaitGroup       // For checking that everything has indeed shut down
	shutdown  chan bool             // To make sure everything can shut down
	bot       *tgbotapi.BotAPI      // The actual bot
	c         *config               // Running configuration
	games     map[chatID][]HangGame // Mapping of people to an array of times
	gamesLock *sync.RWMutex         // Lock of this map
}

type config struct {
	Apikey string  // Telegram API key
	Admins []int64 // Bot admins
	Dict string // What to use as dictionary file
}

type chatID = int64
