package ui

import (
	"fmt"
	"strings"

	"crux_tui/internal/features"

	"github.com/charmbracelet/bubbles/textinput"
	tea "github.com/charmbracelet/bubbletea"
)

func (m Model) Init() tea.Cmd {
	return textinput.Blink
}

func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	var cmd tea.Cmd
	var cmds []tea.Cmd

	switch msg := msg.(type) {
	case tea.KeyMsg:
		if m.ShowInspector {
			if msg.String() == "esc" || msg.String() == "enter" {
				m.ShowInspector = false
			}
			return m, nil
		}

		if msg.String() == "/" && !m.TextInput.Focused() {
			m.TextInput.Focus()
			m.TextInput.SetValue("/")
			return m, textinput.Blink
		}

		switch msg.String() {
		case "ctrl+c":
			return m, tea.Quit

		case "enter":
			if m.TextInput.Focused() {
				input := strings.TrimSpace(m.TextInput.Value())

				if strings.HasPrefix(input, "/load ") {
					path := strings.TrimPrefix(input, "/load ")
					m.StartLoading(path)
				} else if strings.HasPrefix(input, "/filter ") {
					query := strings.TrimPrefix(input, "/filter ")
					m.FilterQuery = query
					filteredRows := features.FilterRows(m.AllRows, query)
					m.Table.SetRows(filteredRows)
					m.Table.SetCursor(0)
					m.Logs = append(m.Logs, "• Filter applied: "+query)
				} else if strings.HasPrefix(input, "/export ") {
					path := strings.TrimPrefix(input, "/export ")
					err := features.ExportCSV(path, m.Table.Columns(), m.Table.Rows())
					if err != nil {
						m.Logs = append(m.Logs, " Export failed: "+err.Error())
					} else {
						m.Logs = append(m.Logs, " Exported to "+path)
					}
				} else if input == "/reset" {
					m.FilterQuery = ""
					m.Table.SetRows(m.AllRows)
					m.Logs = append(m.Logs, "• View reset")
				}

				m.TextInput.SetValue("")
			} else if m.ShowTable {
				m.ShowInspector = true
			}

		case "s":
			if m.ShowTable && !m.TextInput.Focused() {
				if m.SortColumn < 0 {
					m.SortColumn = 0
				}
				m.SortDesc = !m.SortDesc
				currentRows := m.Table.Rows()
				sortedRows := features.SortRows(currentRows, m.SortColumn, m.SortDesc)
				m.Table.SetRows(sortedRows)
				direction := "ASC"
				if m.SortDesc {
					direction = "DESC"
				}
				colName := "unknown"
				if m.SortColumn < len(m.Table.Columns()) {
					colName = m.Table.Columns()[m.SortColumn].Title
				}
				m.Logs = append(m.Logs, fmt.Sprintf("• Sorted %s (%s)", colName, direction))
			}

		case "esc":
			if m.TextInput.Focused() {
				m.TextInput.Blur()
			} else {
				m.TextInput.Focus()
			}

		case "down", "j":
			if m.ShowTable && !m.TextInput.Focused() {
				m.Table, cmd = m.Table.Update(msg)
				cmds = append(cmds, cmd)

				if m.FilterQuery == "" && m.Table.Cursor() >= len(m.AllRows)-5 {
					m.LoadNextBatch()
				}
				return m, tea.Batch(cmds...)
			}
		}

	case tea.WindowSizeMsg:
		m.Width = msg.Width
		m.Height = msg.Height
		m.Ready = true

		tableHeight := m.Height - 8
		if tableHeight < 5 {
			tableHeight = 5
		}
		m.Table.SetHeight(tableHeight)

		if len(m.Table.Columns()) > 0 {
			totalW := m.Width - 35
			perCol := totalW / len(m.Table.Columns())
			if perCol < 10 {
				perCol = 10
			}
			cols := m.Table.Columns()
			for i := range cols {
				cols[i].Width = perCol
			}
			m.Table.SetColumns(cols)
		}
	}

	m.TextInput, cmd = m.TextInput.Update(msg)
	cmds = append(cmds, cmd)

	if m.ShowTable && !m.ShowInspector {
		m.Table, cmd = m.Table.Update(msg)
		cmds = append(cmds, cmd)
	}

	return m, tea.Batch(cmds...)
}
