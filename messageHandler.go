package main

import (
	"fmt"
	"github.com/go-telegram-bot-api/telegram-bot-api"
	"strings"
	"unicode"
)

func messageSender(id int) {
	defer g.wg.Done()
	logInfo.Printf("Starting message sender %d\n", id)
	defer logWarn.Printf("Stopping message sender %d\n", id)

outer:
	for {
		select {
		case <-g.shutdown:
			break outer
		case message := <-sendchannel:
			g.bot.Send(message)
		}
	}
}

func messageHandler() {
	defer g.wg.Done()
	logInfo.Println("Starting message monitor")
	defer logWarn.Println("Stopping message monitor")

	u := tgbotapi.NewUpdate(0)
	u.Timeout = 300

	updates, err := g.bot.GetUpdatesChan(u)
	if err != nil {
		logErr.Printf("Getting updates channel failed: %v\n", err)
	}

outer:
	for {
		select {
		case <-g.shutdown:
			break outer
		case update := <-updates:
			if update.Message == nil {
				continue
			}
			if update.Message.IsCommand() {
				handleCommand(update.Message)
			}
			handleMessage(update.Message)
		}
	}
}

func commandIsForMe(t string) bool {
	command := strings.SplitN(t, " ", 2)[0] // Return first substring before space, this is entire command

	i := strings.Index(command, "@") // Position of @ in command
	if i == -1 {                     // Not in command
		return true // Assume command is for everybody, including this bot
	}

	return strings.ToLower(command[i+1:]) == strings.ToLower(g.bot.Self.UserName)
}

func handleCommand(m *tgbotapi.Message) {
	if !commandIsForMe(m.Text) {
		return
	}

	switch strings.ToLower(m.Command()) {
	case "id":
		handleGetID(m)
	case "hi":
		handleHi(m)
	case "start":
		handleStart(m)
	case "stop":
		handleStop(m)
	case "ping":
		handlePing(m)
	case "pong":
		handlePong(m)
	case "help":
		fallthrough
	default:
		handleHelp(m)
	}
}

func handleMessage(m *tgbotapi.Message) {
	handleChar(m)
}

func handleChar(m *tgbotapi.Message) {
	me, _ := g.bot.GetMe()
	if m.ReplyToMessage != nil && m.ReplyToMessage.From.ID != me.ID {
		// Is not a reply to me
		return
	}
	runes := []rune(m.Text)
	guessed := unicode.ToUpper(runes[0])
	if len(runes) > 1 {
		// Is not "just a letter"
		return
	}
	if guessed < 64 || guessed > 91 {
		// Character outside of range, is not a potential character
		return
	}
	g.gamesLock.Lock()
	defer g.gamesLock.Unlock()

	chatGames := g.games[m.Chat.ID]
	if len(chatGames) == 0 || chatGames[len(chatGames)-1].status != Active {
		// No active game
		return
	}

	game := &chatGames[len(chatGames)-1] // get active game

	if contains(game.guessedChars, guessed) {
		// This character was already tried, ignore
		return
	}

	game.guessedChars = append(game.guessedChars, guessed)

	var guesstimate string
	if contains([]rune(game.secretWord), guessed) {
		guesstimate = "Goede"
	} else {
		guesstimate = "Verkeerde"
	}
	message := tgbotapi.NewMessage(m.Chat.ID, fmt.Sprintf("%s letter!\nWoord:\n%v\nLevens:%d", guesstimate, getObfuscatedWord(game), game.livesLeft))
	message.ReplyToMessageID = m.MessageID
	sendchannel <- message
}

func getObfuscatedWord(game *HangGame) (s string) {
	var mask = '➖'
	var obfuscatedWord = make([]rune, 0, len(game.secretWord))
	for _, r := range []rune(game.secretWord) {
		if contains(game.guessedChars, r) {
			obfuscatedWord = append(obfuscatedWord, r)
		} else {
			obfuscatedWord = append(obfuscatedWord, mask)
		}
	}
	return string(obfuscatedWord)
}

func contains(haystack []rune, needle rune) bool {
	for _, b := range haystack {
		if needle == b {
			return false
		}
	}
	return true
}

func handleStart(m *tgbotapi.Message) {
	g.gamesLock.Lock()
	defer g.gamesLock.Unlock()

	// Check if a game is already active
	games := g.games[m.Chat.ID]

	if games == nil || len(games) == 0 {
		g.games[m.Chat.ID] = make([]HangGame, 0)
		games = g.games[m.Chat.ID]
	} else if games[len(games)-1].status == Active {
		sendchannel <- tgbotapi.NewMessage(m.Chat.ID, "A game is already active!")
	} else {
		g.games[m.Chat.ID] = append(g.games[m.Chat.ID], NewHangGame())
	}

}

func handleStop(m *tgbotapi.Message) {
	g.gamesLock.Lock()
	defer g.gamesLock.Unlock()

	// Check there is a game active, otherwise we cannot stop
	games := g.games[m.Chat.ID]
	if len(games) == 0 || games[len(games)-1].status != Active {
		sendchannel <- tgbotapi.NewMessage(m.Chat.ID, "You cannot stop, there is no game active")
	} else {
		// Stop current game
		var game = &games[len(games)-1]
		if game.livesLeft > 3 {
			game.status = Stopped
			sendchannel <- tgbotapi.NewMessage(m.Chat.ID, fmt.Sprintf("Game has been stopped, you did not lose. The word was: %s.", game.secretWord))
		} else {
			// At this point, you've already spent so much time with this game, you've lost
			game.status = Lost
			sendchannel <- tgbotapi.NewMessage(m.Chat.ID, fmt.Sprintf("You have lost this game. The word was: %s.", game.secretWord))
		}
	}
}

func handleGetID(cmd *tgbotapi.Message) {
	msg := tgbotapi.NewMessage(
			cmd.Chat.ID,
			fmt.Sprintf(
				"Hi, %s %s, your Telegram user ID is given by %d",
				cmd.From.FirstName,
				cmd.From.LastName,
				cmd.From.ID))
	msg.ReplyToMessageID = cmd.MessageID
	sendchannel <- msg
}

func handleHelp(m *tgbotapi.Message) {
	sendchannel <- tgbotapi.NewMessage(m.Chat.ID, "You can play Hangman with this bot. Try it out using /start!")
}
func handleHi(m *tgbotapi.Message) {
	sendchannel <- tgbotapi.NewMessage(m.Chat.ID, "Hi!")
}
func handlePing(m *tgbotapi.Message) {
	sendchannel <- tgbotapi.NewMessage(m.Chat.ID, "Pong!")
}
func handlePong(m *tgbotapi.Message) {
	sendchannel <- tgbotapi.NewMessage(m.Chat.ID, "Ping!")
}
