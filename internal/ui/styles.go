package ui

import "github.com/charmbracelet/lipgloss"

var (
	ColorPrimary   = lipgloss.Color("#2196F3")
	ColorSecondary = lipgloss.Color("#FF9800")
	ColorSubtle    = lipgloss.Color("#6272A4")
	ColorText      = lipgloss.Color("#F8F8F2")
	ColorDark      = lipgloss.Color("#282A36")
	ColorSuccess   = lipgloss.Color("#50FA7B")
	ColorError     = lipgloss.Color("#FF5555")

	TitleStyle = lipgloss.NewStyle().
			Foreground(ColorPrimary).
			Bold(true).
			MarginLeft(1)

	SubtleStyle = lipgloss.NewStyle().
			Foreground(ColorSubtle)

	MainContainerStyle = lipgloss.NewStyle().
				Margin(0)

	SidebarStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder(), false, true, false, false).
			BorderForeground(ColorSubtle).
			Padding(0, 1)

	MenuItemStyle = lipgloss.NewStyle().
			Foreground(ColorText).
			PaddingLeft(1)

	MenuHeaderStyle = lipgloss.NewStyle().
			Foreground(ColorSecondary).
			Bold(true).
			Padding(1, 0, 0, 0)

	ContentStyle = lipgloss.NewStyle().
			Padding(0, 1)

	EmptyStateStyle = lipgloss.NewStyle().
			Foreground(ColorSubtle).
			Italic(true)

	StatusBarStyle = lipgloss.NewStyle().
			Foreground(ColorText).
			Background(lipgloss.Color("#343746"))

	StatusTextItemStyle = lipgloss.NewStyle().
				Foreground(ColorText).
				Background(ColorPrimary).
				Padding(0, 1).
				Bold(true)

	StatusModeStyle = lipgloss.NewStyle().
			Foreground(lipgloss.Color("#1B1D25")).
			Background(ColorSecondary).
			Padding(0, 1).
			Bold(true)

	InputBoxStyle = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorPrimary).
			Padding(0, 1)
)
