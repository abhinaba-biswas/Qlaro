package features

import (
	"sort"

	"github.com/charmbracelet/bubbles/table"
)

func SortRows(rows []table.Row, colIndex int, descending bool) []table.Row {

	sort.SliceStable(rows, func(i, j int) bool {
		valA := rows[i][colIndex]
		valB := rows[j][colIndex]

		if descending {
			return valA > valB
		}
		return valA < valB
	})

	return rows
}
