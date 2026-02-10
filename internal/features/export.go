package features

import (
	"encoding/csv"
	"os"

	"github.com/charmbracelet/bubbles/table"
)

func ExportCSV(filename string, cols []table.Column, rows []table.Row) error {
	f, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	writer := csv.NewWriter(f)
	defer writer.Flush()

	headerRow := make([]string, len(cols))
	for i, c := range cols {
		headerRow[i] = c.Title
	}
	if err := writer.Write(headerRow); err != nil {
		return err
	}

	for _, r := range rows {
		if err := writer.Write(r); err != nil {
			return err
		}
	}

	return nil
}
