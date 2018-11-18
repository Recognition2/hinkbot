package main

import (
	"io/ioutil"
	"math/rand"
	"strings"
)

const dict string = "/usr/share/dict/words"

const (
	Active HangGameStatus = iota
	Stopped
	Won
	Lost
)

type HangGameStatus int8
type HangGame struct {
	status       HangGameStatus // Is a game currently being played?
	guessedChars []rune         // What letters have been tried yet
	livesLeft    int
	secretWord   string // This HangGame, we try to guess this word
}

func getRandomWordFromDict(s string) string {
	words, err := ioutil.ReadFile(s)
	if err != nil {
		logErr.Println(err)
	}
	wordsList := strings.Split(string(words), "\n")
	newWordList := make([]string, 0, len(wordsList))
	for _, w := range wordsList {
		if strings.Contains(w, "'") ||
			len(w) <= 4 {
			continue
		}
		newWordList = append(newWordList, w)
	}
	return newWordList[rand.Int31n(int32(len(newWordList)))]
}

func NewHangGame() HangGame {
	// First get a new word
	randomWord := getRandomWordFromDict(dict)

	return HangGame{status: Active,
		livesLeft:    10,
		secretWord:   randomWord,
		guessedChars: make([]rune, 0, 26)}
}
