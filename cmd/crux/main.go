package main

import (
	"fmt"
	"os"

	"crux_tui/internal/ui"

	tea "github.com/charmbracelet/bubbletea"
)

func main() {

	initialModel := ui.InitialModel()

	p := tea.NewProgram(
		initialModel,
		tea.WithAltScreen(),
		tea.WithMouseCellMotion(),
	)

	if _, err := p.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "CRUX FATAL ERROR: %v\n", err)
		os.Exit(1)
	}
}
