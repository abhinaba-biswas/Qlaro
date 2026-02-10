package ui

import (
	"fmt"
	"strings"

	"crux_tui/internal/features"

	"github.com/charmbracelet/lipgloss"
)

func (m Model) View() string {
	if !m.Ready {
		return "Initializing..."
	}

	if m.ShowInspector {
		return features.ViewInspector(
			m.Width,
			m.Height,
			m.Table.SelectedRow(),
			m.Table.Columns(),
		)
	}

	header := lipgloss.NewStyle().
		Foreground(lipgloss.Color("#2196F3")).
		Bold(true).
		Margin(1, 0, 1, 2).
		Render("CRUX TUI")

	displayLogs := m.Logs
	if len(m.Logs) > 10 {
		displayLogs = m.Logs[len(m.Logs)-10:]
	}

	logView := ""
	for _, l := range displayLogs {
		if len(logView) > 0 {
			logView += "\n"
		}
		if strings.Contains(l, "x") {
			logView += lipgloss.NewStyle().Foreground(lipgloss.Color("#FF5555")).Render(l)
		} else {
			logView += lipgloss.NewStyle().Foreground(lipgloss.Color("240")).Render(l)
		}
	}

	sidebar := lipgloss.NewStyle().
		Width(30).
		Height(m.Height-5).
		Border(lipgloss.NormalBorder(), false, true, false, false).
		BorderForeground(lipgloss.Color("240")).
		Padding(0, 1).
		Render(
			lipgloss.JoinVertical(lipgloss.Left,
				lipgloss.NewStyle().Bold(true).Render("COMMANDS"),
				"/load <file>",
				"/filter <txt>",
				"/export <file>",
				"/reset",
				"/exit",
				"\n",
				lipgloss.NewStyle().Bold(true).Render("SYSTEM LOGS"),
				logView,
			),
		)

	var content string
	if m.ShowTable {
		content = lipgloss.NewStyle().
			MarginLeft(1).
			Render(m.Table.View())
	} else {
		content = lipgloss.Place(
			m.Width-35, m.Height-5,
			lipgloss.Center, lipgloss.Center,
			lipgloss.NewStyle().Foreground(lipgloss.Color("240")).Render("NO DATA LOADED\nUse /load <file.csv> to begin"),
		)
	}
	// I am commet out the colours if you want to make chages you can
	modeLabel := " COMMAND "
	modeColor := lipgloss.Color("57") // Indigo

	if m.FilterQuery != "" {
		modeLabel = " FILTERED "
		modeColor = lipgloss.Color("#FF9800") // Orange
	}

	statusInfo := fmt.Sprintf(" %s | Rows: %d ", m.DatasetName, len(m.Table.Rows()))

	bar := lipgloss.JoinHorizontal(lipgloss.Top,
		lipgloss.NewStyle().Background(modeColor).Foreground(lipgloss.Color("255")).Bold(true).Render(modeLabel),
		lipgloss.NewStyle().Background(lipgloss.Color("240")).Foreground(lipgloss.Color("255")).Render(statusInfo),
		" ",
		m.TextInput.View(),
	)

	body := lipgloss.JoinHorizontal(lipgloss.Top, sidebar, content)
	return lipgloss.JoinVertical(lipgloss.Left, header, body, "\n"+bar)
}
