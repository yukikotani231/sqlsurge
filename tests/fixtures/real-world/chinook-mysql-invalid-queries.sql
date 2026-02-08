-- Invalid queries against the Chinook MySQL schema (error detection tests)

-- E0001: Table not found
SELECT * FROM Song;

-- E0002: Column not found (typo in Artist)
SELECT ArtistId, Nme FROM Artist;

-- E0002: Column not found (wrong column)
SELECT TrackId, SongName FROM Track;

-- E0001: Table not found in JOIN
SELECT al.Title, l.Name
FROM Album al
INNER JOIN Label l ON al.LabelId = l.LabelId;

-- E0002: Column not found in JOIN condition
SELECT t.Name, al.Title
FROM Track t
INNER JOIN Album al ON t.album_id = al.AlbumId;

-- E0002: Column not found in WHERE
SELECT FirstName, LastName FROM Customer WHERE Age > 30;

-- E0002: Column not found in aggregate
SELECT GenreId, AVG(Duration) FROM Track GROUP BY GenreId;

-- E0005: Column count mismatch in INSERT
INSERT INTO Artist (Name) VALUES ('Artist1', 'Artist2');

-- E0002: Column not found in INSERT
INSERT INTO Track (Name, AlbumId, MediaTypeId, GenreId, Length, UnitPrice)
VALUES ('Test', 1, 1, 1, 300000, 0.99);

-- E0001: Table not found in subquery
SELECT Name FROM Artist
WHERE ArtistId IN (SELECT ArtistId FROM Record);

-- E0001: Table not found in DELETE
DELETE FROM Songs WHERE TrackId = 1;

-- E0002: Column not found in UPDATE
UPDATE Album SET AlbumTitle = 'New Title' WHERE AlbumId = 1;
