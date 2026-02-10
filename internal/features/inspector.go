package features

import (
	"fmt"
	"strings"

	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/lipgloss"
)

func ViewInspector(width, height int, row table.Row, cols []table.Column) string {
	if row == nil {
		return ""
	}

	var details strings.Builder

	details.WriteString(lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("#2196F3")).
		Render("ROW INSPECTOR") + "\n\n")

	maxLabelW := 0
	for _, col := range cols {
		if len(col.Title) > maxLabelW {
			maxLabelW = len(col.Title)
		}
	}

	for i, val := range row {
		if i < len(cols) {
			label := fmt.Sprintf("%-*s", maxLabelW, cols[i].Title)
			styledLabel := lipgloss.NewStyle().Foreground(lipgloss.Color("240")).Render(label)
			styledValue := lipgloss.NewStyle().Foreground(lipgloss.Color("255")).Render(val)
			details.WriteString(fmt.Sprintf("%s : %s\n", styledLabel, styledValue))
		}
	}

	details.WriteString("\n" + lipgloss.NewStyle().
		Italic(true).
		Foreground(lipgloss.Color("240")).
		Render("Press Esc to close"))

	popup := lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("#2196F3")).
		Padding(1, 2).
		Background(lipgloss.Color("#1B1D25")).
		Width(60).
		Render(details.String())

	return lipgloss.Place(
		width, height,
		lipgloss.Center, lipgloss.Center,
		popup,
		lipgloss.WithWhitespaceChars(" "),
		lipgloss.WithWhitespaceForeground(lipgloss.NoColor{}),
	)
}
