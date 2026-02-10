package ui

import (
	"encoding/csv"
	"os"
	"path/filepath"
	"strings"

	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/lipgloss"
)

const BatchSize = 100

func (m *Model) StartLoading(path string) {
	f, err := os.Open(path)
	if err != nil {
		m.Logs = append(m.Logs, "✖ Error: "+err.Error())
		return
	}

	m.File = f
	m.CSVReader = csv.NewReader(f)
	m.DatasetName = filepath.Base(path)

	headers, err := m.CSVReader.Read()
	if err != nil {
		m.Logs = append(m.Logs, " Invalid CSV")
		return
	}

	cols := make([]table.Column, len(headers))
	for i, h := range headers {
		cols[i] = table.Column{Title: strings.ToUpper(h), Width: 15}
	}
	m.Table.SetColumns(cols)

	m.AllRows = []table.Row{}
	m.TotalLoaded = 0

	m.LoadNextBatch()

	m.ShowTable = true
	m.Logs = append(m.Logs, " Loaded: "+m.DatasetName)
	m.TextInput.Blur()

	s := table.DefaultStyles()
	s.Header = s.Header.
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(lipgloss.Color("240")).
		BorderBottom(true).
		Bold(true)
	s.Selected = s.Selected.
		Foreground(lipgloss.Color("229")).
		Background(lipgloss.Color("57")).
		Bold(false)
	m.Table.SetStyles(s)
}

func (m *Model) LoadNextBatch() {
	if m.CSVReader == nil {
		return
	}

	for i := 0; i < BatchSize; i++ {
		record, err := m.CSVReader.Read()
		if err != nil {
			m.File.Close()
			m.CSVReader = nil
			m.Logs = append(m.Logs, "• End of File reached")
			break
		}
		m.AllRows = append(m.AllRows, table.Row(record))
	}

	m.Table.SetRows(m.AllRows)
	m.TotalLoaded = len(m.AllRows)
}
