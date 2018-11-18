package main

import (
	"encoding/json"
	"github.com/BurntSushi/toml"
	"github.com/go-telegram-bot-api/telegram-bot-api"
	"log"
	"math/rand"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"
)

var (
	logErr  = log.New(os.Stderr, "[ERRO] ", log.Ldate+log.Ltime+log.Ltime+log.Lshortfile)
	logWarn = log.New(os.Stdout, "[WARN] ", log.Ldate+log.Ltime)
	logInfo = log.New(os.Stdout, "[INFO] ", log.Ldate+log.Ltime)
	g       = global{shutdown: make(chan bool),
		games: make(map[chatID][]HangGame)}
	sendMessageChan = make(chan tgbotapi.MessageConfig, 128)
)

func main() {
	/////////////
	// STARTUP
	//////////////

	// Parse settings file
	_, err := toml.DecodeFile("settings.toml", &g.c)
	if err != nil {
		logErr.Println(err)
		return
	}

	// Seed the RNG
	rand.Seed(time.Now().UnixNano())

	// Create new bot
	g.bot, err = tgbotapi.NewBotAPI(g.c.Apikey)
	if err != nil {
		logErr.Println(err)
	}

	logInfo.Printf("Running as @%s", g.bot.Self.UserName)

	// Create waitgroup, for synchronized shutdown
	var wg sync.WaitGroup
	g.wg = &wg

	// Create the lock for the stats object
	var gamesLock sync.RWMutex
	g.gamesLock = &gamesLock

	// Fill subscriptions object
	err = Load("data.gob", &g.games)
	if err != nil {
		logErr.Println(err)
	}

	// All messages are received by the messageHandler
	wg.Add(1)
	go messageHandler()

	for i := 0; i < 3; i++ { // Start 3 async message senders
		wg.Add(1)
		go messageSender(i)
	}

	wg.Add(1)
	go dataSaver()

	// Perform other startup tasks

	sigs := make(chan os.Signal, 2)
	signal.Notify(sigs, os.Interrupt, syscall.SIGINT)

	time.Sleep(time.Millisecond)
	logInfo.Println("All routines have been started, awaiting kill signal")

	///////////////
	// SHUTDOWN
	///////////////

	// Program will hang here
	// On this select statement

	select {
	case <-sigs:
		close(g.shutdown)
	case <-g.shutdown:
	}
	println()
	logInfo.Println("Shutdown signal received. Waiting for goroutines")

	// Shutdown after all goroutines have exited
	g.wg.Wait()
	logWarn.Println("Shutting down")
}

func Save(path string, object interface{}) error {
	file, err := os.Create(path)
	if err == nil {
		encoder := json.NewEncoder(file)
		encoder.Encode(object)
	}
	file.Close()
	return err
}
func Load(path string, o interface{}) error {
	file, err := os.Open(path)
	if err == nil {
		dec := json.NewDecoder(file)
		err = dec.Decode(o)
	}
	file.Close()
	return err
}
