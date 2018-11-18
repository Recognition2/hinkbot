package main

import (
	"io/ioutil"
	"math/rand"
	"strings"
)

//const dict string = "/home/gregory/dict_nl.txt"

const (
	Active HangGameStatus = iota
	Stopped
	Won
	Lost
)

type Guess struct {
	Author  int
	Char    rune
	Correct bool
}

type HangGameStatus int8
type HangGame struct {
	Status     HangGameStatus // Is a game currently being played?
	Guesses    []Guess
	LivesLeft  int
	SecretWord string // This HangGame, we try to guess this word
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
	return strings.ToUpper(newWordList[rand.Int31n(int32(len(newWordList)-1))])
}

func NewHangGame() HangGame {
	return HangGame{Status: Active,
		LivesLeft:  10,
		SecretWord: getRandomWordFromDict(g.c.Dict),
		Guesses:    make([]Guess, 0, 26)}
}
