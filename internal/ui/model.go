package ui

import (
	"encoding/csv"
	"os"

	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/bubbles/textinput"
)

type Model struct {
	Width  int
	Height int
	Ready  bool

	TextInput textinput.Model
	Table     table.Model

	ShowTable     bool
	ShowInspector bool

	File        *os.File
	CSVReader   *csv.Reader
	AllRows     []table.Row
	TotalLoaded int
	DatasetName string

	FilterQuery string
	SortColumn  int
	SortDesc    bool

	Logs []string
}

func InitialModel() Model {
	ti := textinput.New()
	ti.Placeholder = "Type /load <filename>..."
	ti.Focus()
	ti.CharLimit = 156
	ti.Width = 50

	t := table.New(
		table.WithColumns([]table.Column{{Title: "Waiting...", Width: 20}}),
		table.WithFocused(true),
		table.WithHeight(10),
	)

	return Model{
		TextInput:   ti,
		Table:       t,
		DatasetName: "No Dataset",
		Logs:        []string{"• System Initialized"},

		ShowTable:     false,
		ShowInspector: false,
		Ready:         false,

		FilterQuery: "",
		SortColumn:  -1,
		SortDesc:    false,
	}
}
