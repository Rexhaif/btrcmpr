# Which file passed at each preset?

**Pass** means the Rust archive was smaller, encode was faster, and decode reached at least 95% of zstd. Otherwise **S** = size failed, **E** = encode failed, **D** = decode fell below 95%. A dash means the cell is pending. Click a cell to see the five original pairs. [Absolute measurements and aggregate totals](README.md).

| Dataset | Source | -1 | -3 | -10 | -22 |
|---|---|---:|---:|---:|---:|
| alice29.txt | Canterbury Corpus | [SE](raw/01-alice29.txt.csv) | [SED](raw/03-alice29.txt.csv) | [SE](raw/10-alice29.txt.csv) | [SE](raw/22-alice29.txt.csv) |
| asyoulik.txt | Canterbury Corpus | [SE](raw/01-asyoulik.txt.csv) | [SE](raw/03-asyoulik.txt.csv) | [SE](raw/10-asyoulik.txt.csv) | [SE](raw/22-asyoulik.txt.csv) |
| cp.html | Canterbury Corpus | [SE](raw/01-cp.html.csv) | [SED](raw/03-cp.html.csv) | [SED](raw/10-cp.html.csv) | [SED](raw/22-cp.html.csv) |
| dickens | Silesia Corpus | [SD](raw/01-dickens.csv) | [S](raw/03-dickens.csv) | [S](raw/10-dickens.csv) | [D](raw/22-dickens.csv) |
| fields.c | Canterbury Corpus | [SED](raw/01-fields.c.csv) | [SED](raw/03-fields.c.csv) | [SED](raw/10-fields.c.csv) | [SED](raw/22-fields.c.csv) |
| fireworks.jpeg | Snappy | [ED](raw/01-fireworks.jpeg.csv) | [ED](raw/03-fireworks.jpeg.csv) | [ED](raw/10-fireworks.jpeg.csv) | [SED](raw/22-fireworks.jpeg.csv) |
| geo.protodata | Snappy | [SED](raw/01-geo.protodata.csv) | [SED](raw/03-geo.protodata.csv) | [SED](raw/10-geo.protodata.csv) | [SD](raw/22-geo.protodata.csv) |
| grammar.lsp | Canterbury Corpus | [SED](raw/01-grammar.lsp.csv) | [SE](raw/03-grammar.lsp.csv) | [SE](raw/10-grammar.lsp.csv) | [SE](raw/22-grammar.lsp.csv) |
| kennedy.xls | Canterbury Corpus | [SE](raw/01-kennedy.xls.csv) | [SED](raw/03-kennedy.xls.csv) | [SE](raw/10-kennedy.xls.csv) | [S](raw/22-kennedy.xls.csv) |
| lcet10.txt | Canterbury Corpus | [SED](raw/01-lcet10.txt.csv) | [E](raw/03-lcet10.txt.csv) | [SED](raw/10-lcet10.txt.csv) | [S](raw/22-lcet10.txt.csv) |
| mr | Silesia Corpus | [SE](raw/01-mr.csv) | [**Pass**](raw/03-mr.csv) | [E](raw/10-mr.csv) | [SE](raw/22-mr.csv) |
| nci | Silesia Corpus | [SE](raw/01-nci.csv) | [SE](raw/03-nci.csv) | [SE](raw/10-nci.csv) | [S](raw/22-nci.csv) |
| ooffice | Silesia Corpus | [SE](raw/01-ooffice.csv) | [**Pass**](raw/03-ooffice.csv) | [SE](raw/10-ooffice.csv) | [SE](raw/22-ooffice.csv) |
| osdb | Silesia Corpus | [D](raw/01-osdb.csv) | [**Pass**](raw/03-osdb.csv) | [SE](raw/10-osdb.csv) | [SE](raw/22-osdb.csv) |
| paper-100k.pdf | Snappy | [E](raw/01-paper-100k.pdf.csv) | [ED](raw/03-paper-100k.pdf.csv) | [ED](raw/10-paper-100k.pdf.csv) | [SD](raw/22-paper-100k.pdf.csv) |
| plrabn12.txt | Canterbury Corpus | [E](raw/01-plrabn12.txt.csv) | [E](raw/03-plrabn12.txt.csv) | [SE](raw/10-plrabn12.txt.csv) | [S](raw/22-plrabn12.txt.csv) |
| ptt5 | Canterbury Corpus | [SED](raw/01-ptt5.csv) | [SED](raw/03-ptt5.csv) | [SED](raw/10-ptt5.csv) | [SD](raw/22-ptt5.csv) |
| reymont | Silesia Corpus | [S](raw/01-reymont.csv) | [E](raw/03-reymont.csv) | [SE](raw/10-reymont.csv) | [SE](raw/22-reymont.csv) |
| samba | Silesia Corpus | [SE](raw/01-samba.csv) | [S](raw/03-samba.csv) | [SE](raw/10-samba.csv) | [SE](raw/22-samba.csv) |
| sao | Silesia Corpus | [ED](raw/01-sao.csv) | [D](raw/03-sao.csv) | [E](raw/10-sao.csv) | [SE](raw/22-sao.csv) |
| sum | Canterbury Corpus | [SED](raw/01-sum.csv) | [SED](raw/03-sum.csv) | [SED](raw/10-sum.csv) | [SD](raw/22-sum.csv) |
| urls.10K | Snappy | [SED](raw/01-urls.10K.csv) | [ED](raw/03-urls.10K.csv) | [SED](raw/10-urls.10K.csv) | [S](raw/22-urls.10K.csv) |
| xargs.1 | Canterbury Corpus | [SED](raw/01-xargs.1.csv) | [SED](raw/03-xargs.1.csv) | [SED](raw/10-xargs.1.csv) | [SE](raw/22-xargs.1.csv) |
| webster | Silesia Corpus | [S](raw/01-webster.csv) | [**Pass**](raw/03-webster.csv) | [S](raw/10-webster.csv) | [D](raw/22-webster.csv) |
| xml | Silesia Corpus | [S](raw/01-xml.csv) | [SE](raw/03-xml.csv) | [SED](raw/10-xml.csv) | [S](raw/22-xml.csv) |
