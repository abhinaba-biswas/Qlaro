package features

import (
	"strings"

	"github.com/charmbracelet/bubbles/table"
)

func FilterRows(allRows []table.Row, query string) []table.Row {
	if query == "" {
		return allRows
	}

	var filtered []table.Row
	query = strings.ToLower(query)

	for _, row := range allRows {
		match := false
		for _, col := range row {
			if strings.Contains(strings.ToLower(col), query) {
				match = true
				break
			}
		}
		if match {
			filtered = append(filtered, row)
		}
	}
	return filtered
}
