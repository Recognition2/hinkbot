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
		case message := <-sendMessageChan:
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
			} else {
				handleMessage(update.Message)
			}
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
	case "stats":
		handleStats(m)
	case "help":
		fallthrough
	default:
		handleHelp(m)
	}
}

func handleStats(m *tgbotapi.Message) {
	g.gamesLock.RLock()
	defer g.gamesLock.RUnlock()

	games := g.games[m.Chat.ID]
	if games == nil {
		sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "There are no stats for this chat yet :(")
		return
	}

	var msg = ""
	var gamesWon int
	var gamesLost int
	for _, game := range games {
		if game.Status == Won {
			gamesWon += 1
		} else if game.Status == Lost {
			gamesLost += 1
		}
	}
	msg += fmt.Sprintf("Games won: %d\n", gamesWon)
	msg += fmt.Sprintf("Games lost: %d\n", gamesLost)

	sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, msg)
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
	if len(runes) > 1 {
		// Is not "just a letter"
		return
	}
	guessed := unicode.ToUpper(runes[0])
	if guessed < 64 || guessed > 91 {
		// Character outside of range, is not a potential character
		return
	}

	g.gamesLock.Lock()
	defer g.gamesLock.Unlock()

	chatGames := g.games[m.Chat.ID]
	if len(chatGames) == 0 || chatGames[len(chatGames)-1].Status != Active {
		// No active game
		return
	}

	game := &chatGames[len(chatGames)-1] // get active game

	if guessHasBeenTried(game.Guesses, guessed) {
		// This character was already tried, ignore
		return
	}
	logInfo.Printf("The first letter of your response is %c\n", guessed)
	guessIsCorrect := contains([]rune(game.SecretWord), guessed)
	game.Guesses = append(game.Guesses, Guess{Author: m.From.ID, Char:guessed, Correct: guessIsCorrect})

	var txt string
	if guessedCompleteWord(game) {
		// THE GAME HAS BEEN WON
		txt = fmt.Sprintf("Gefeliciteerd! Je hebt gewonnen! Het woord was: %s\nDruk op /start om opnieuw te beginnen", game.SecretWord)
		game.Status = Won
	} else if contains([]rune(game.SecretWord), guessed) {
		// Correct letter guess
		txt = fmt.Sprintf("Goede letter!\nWoord: \n%v\nLevens: %d", getObfuscatedWord(game), game.LivesLeft)
	} else if game.LivesLeft > 1 {
		// Incorrect letter guess
		game.LivesLeft -= 1
		txt = fmt.Sprintf("Verkeerde letter!\nWoord: \n%v\nLevens: %d", getObfuscatedWord(game), game.LivesLeft)
	} else {
		// Game has been lost
		txt = fmt.Sprintf("Je hebt verloren :(\nHet woord was: \n%v\nDruk op /start om opnieuw te beginnen", game.SecretWord)
		game.Status = Lost
	}
	var msg = tgbotapi.NewMessage(m.Chat.ID, txt)
	if game.Status == Active {
		msg.ReplyMarkup = tgbotapi.ForceReply{ForceReply: true}

	}
	msg.ReplyToMessageID = m.MessageID
	sendMessageChan <- msg
}

func guessHasBeenTried(haystack []Guess, r rune) bool {
	for _,g := range haystack {
		if g.Char == r {
			return true
		}
	}
	return false
}

func guessedCompleteWord(game *HangGame) bool {
	for _, r := range []rune(game.SecretWord) {
		if !guessHasBeenTried(game.Guesses, r){
			return false
		}
	}
	return true
}

func getObfuscatedWord(game *HangGame) (s string) {
	var mask = '➖'
	var obfuscatedWord = make([]rune, 0, len(game.SecretWord))
	for _, r := range []rune(game.SecretWord) {
		if guessHasBeenTried(game.Guesses, r) {
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
			return true
		}
	}
	return false
}

func handleStart(m *tgbotapi.Message) {
	g.gamesLock.Lock()
	defer g.gamesLock.Unlock()

	// Check if a game is already active
	games := g.games[m.Chat.ID]
	fmt.Printf(">HandleStart: Games is %+v\n", games)

	if len(games) != 0 && games[len(games)-1].Status == Active {
		sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "A game is already active!")
		return
	}
	if games == nil {
		games = make([]HangGame, 0, 1)
	}

	games = append(games, NewHangGame())
	game := &games[len(games)-1]

	msg := tgbotapi.NewMessage(
		m.Chat.ID,
		fmt.Sprintf(
			"Een nieuw spel is gestart!\nWoord: %s\nLevens: %d",
			getObfuscatedWord(
				game),
			game.LivesLeft))
	msg.ReplyToMessageID = m.MessageID
	msg.ReplyMarkup = tgbotapi.ForceReply{ForceReply: true, Selective: false}
	sendMessageChan <- msg

	logInfo.Printf("The secret word is %s\n", game.SecretWord)

	// Restore
	g.games[m.Chat.ID] = games
}

func handleStop(m *tgbotapi.Message) {
	g.gamesLock.Lock()
	defer g.gamesLock.Unlock()

	// Check there is a game active, otherwise we cannot stop
	games := g.games[m.Chat.ID]
	if len(games) == 0 || games[len(games)-1].Status != Active {
		sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "You cannot stop, there is no game active")
	} else {
		// Stop current game
		var game = &games[len(games)-1]
		if game.LivesLeft > 3 {
			game.Status = Stopped
			sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, fmt.Sprintf("Game has been stopped, you did not lose. The word was: %s.", game.SecretWord))
		} else {
			// At this point, you've already spent so much time with this game, you've lost
			game.Status = Lost
			sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, fmt.Sprintf("You have lost this game. The word was: %s.", game.SecretWord))
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
	sendMessageChan <- msg
}

func handleHelp(m *tgbotapi.Message) {
	sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "You can play Hangman with this bot. Try it out using /start!")
}
func handleHi(m *tgbotapi.Message) {
	sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "Hi!")
}
func handlePing(m *tgbotapi.Message) {
	sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "Pong!")
}
func handlePong(m *tgbotapi.Message) {
	sendMessageChan <- tgbotapi.NewMessage(m.Chat.ID, "Ping!")
}
