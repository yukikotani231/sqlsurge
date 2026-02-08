-- Valid queries against the Chinook MySQL schema
-- Tests: SELECT, JOIN, subquery, CTE, aggregate, INSERT, UPDATE, DELETE

-- 1. Basic SELECT
SELECT ArtistId, Name FROM Artist;

-- 2. SELECT with WHERE
SELECT TrackId, Name, UnitPrice FROM Track WHERE UnitPrice > 0.99;

-- 3. Album with Artist JOIN
SELECT al.Title, ar.Name AS ArtistName
FROM Album al
INNER JOIN Artist ar ON al.ArtistId = ar.ArtistId;

-- 4. Track with Album and Artist
SELECT t.Name AS TrackName, al.Title AS AlbumTitle, ar.Name AS ArtistName
FROM Track t
INNER JOIN Album al ON t.AlbumId = al.AlbumId
INNER JOIN Artist ar ON al.ArtistId = ar.ArtistId;

-- 5. Track with Genre and MediaType
SELECT t.Name, g.Name AS Genre, mt.Name AS MediaType
FROM Track t
INNER JOIN Genre g ON t.GenreId = g.GenreId
INNER JOIN MediaType mt ON t.MediaTypeId = mt.MediaTypeId;

-- 6. LEFT JOIN (artists without albums)
SELECT ar.Name, al.Title
FROM Artist ar
LEFT JOIN Album al ON ar.ArtistId = al.ArtistId;

-- 7. Customer with support rep (Employee self-ref)
SELECT c.FirstName, c.LastName, c.Email,
       e.FirstName AS RepFirstName, e.LastName AS RepLastName
FROM Customer c
LEFT JOIN Employee e ON c.SupportRepId = e.EmployeeId;

-- 8. Employee hierarchy (self-join)
SELECT e.FirstName AS Employee, e.Title,
       m.FirstName AS Manager, m.Title AS ManagerTitle
FROM Employee e
LEFT JOIN Employee m ON e.ReportsTo = m.EmployeeId;

-- 9. Invoice with Customer
SELECT i.InvoiceId, i.InvoiceDate, i.Total,
       c.FirstName, c.LastName, c.Email
FROM Invoice i
INNER JOIN Customer c ON i.CustomerId = c.CustomerId;

-- 10. InvoiceLine with Track details
SELECT il.InvoiceLineId, il.Quantity, il.UnitPrice,
       t.Name AS TrackName, al.Title AS AlbumTitle
FROM InvoiceLine il
INNER JOIN Track t ON il.TrackId = t.TrackId
INNER JOIN Album al ON t.AlbumId = al.AlbumId;

-- 11. Playlist contents
SELECT p.Name AS PlaylistName, t.Name AS TrackName
FROM Playlist p
INNER JOIN PlaylistTrack pt ON p.PlaylistId = pt.PlaylistId
INNER JOIN Track t ON pt.TrackId = t.TrackId;

-- 12. Aggregate: tracks per genre
SELECT g.Name AS Genre, COUNT(t.TrackId) AS TrackCount
FROM Genre g
INNER JOIN Track t ON g.GenreId = t.GenreId
GROUP BY g.Name;

-- 13. Aggregate: revenue per customer
SELECT c.FirstName, c.LastName, SUM(i.Total) AS TotalSpent
FROM Customer c
INNER JOIN Invoice i ON c.CustomerId = i.CustomerId
GROUP BY c.CustomerId, c.FirstName, c.LastName;

-- 14. Aggregate: albums per artist with HAVING
SELECT ar.Name, COUNT(al.AlbumId) AS AlbumCount
FROM Artist ar
INNER JOIN Album al ON ar.ArtistId = al.ArtistId
GROUP BY ar.ArtistId, ar.Name
HAVING COUNT(al.AlbumId) > 5;

-- 15. Aggregate: total duration per album (milliseconds)
SELECT al.Title, SUM(t.Milliseconds) AS TotalDuration, COUNT(t.TrackId) AS TrackCount
FROM Album al
INNER JOIN Track t ON al.AlbumId = t.AlbumId
GROUP BY al.AlbumId, al.Title;

-- 16. Subquery in WHERE (IN)
SELECT Name FROM Artist
WHERE ArtistId IN (
    SELECT al.ArtistId FROM Album al WHERE al.AlbumId IN (
        SELECT t.AlbumId FROM Track t WHERE t.GenreId = 1
    )
);

-- 17. Subquery in WHERE (EXISTS)
SELECT c.FirstName, c.LastName
FROM Customer c
WHERE EXISTS (
    SELECT 1 FROM Invoice i WHERE i.CustomerId = c.CustomerId AND i.Total > 10
);

-- 18. Correlated subquery (scalar)
SELECT ar.Name,
    (SELECT COUNT(*) FROM Album al WHERE al.ArtistId = ar.ArtistId) AS AlbumCount
FROM Artist ar;

-- 19. Subquery in FROM (derived table)
SELECT sub.Genre, sub.AvgPrice
FROM (
    SELECT g.Name AS Genre, AVG(t.UnitPrice) AS AvgPrice
    FROM Genre g
    INNER JOIN Track t ON g.GenreId = t.GenreId
    GROUP BY g.Name
) AS sub
WHERE sub.AvgPrice > 1.00;

-- 20. CTE: top selling tracks
WITH track_sales AS (
    SELECT t.TrackId, t.Name, SUM(il.Quantity) AS TotalSold
    FROM Track t
    INNER JOIN InvoiceLine il ON t.TrackId = il.TrackId
    GROUP BY t.TrackId, t.Name
)
SELECT Name, TotalSold
FROM track_sales
WHERE TotalSold > 2;

-- 21. CTE: customer invoice summary
WITH customer_summary AS (
    SELECT c.CustomerId, c.FirstName, c.LastName,
           COUNT(i.InvoiceId) AS InvoiceCount,
           SUM(i.Total) AS TotalSpent
    FROM Customer c
    INNER JOIN Invoice i ON c.CustomerId = i.CustomerId
    GROUP BY c.CustomerId, c.FirstName, c.LastName
)
SELECT FirstName, LastName, InvoiceCount, TotalSpent
FROM customer_summary;

-- 22. Multiple CTEs
WITH genre_tracks AS (
    SELECT g.GenreId, g.Name AS GenreName, COUNT(t.TrackId) AS TrackCount
    FROM Genre g
    INNER JOIN Track t ON g.GenreId = t.GenreId
    GROUP BY g.GenreId, g.Name
),
genre_revenue AS (
    SELECT g.GenreId, SUM(il.UnitPrice * il.Quantity) AS Revenue
    FROM Genre g
    INNER JOIN Track t ON g.GenreId = t.GenreId
    INNER JOIN InvoiceLine il ON t.TrackId = il.TrackId
    GROUP BY g.GenreId
)
SELECT gt.GenreName, gt.TrackCount, gr.Revenue
FROM genre_tracks gt
INNER JOIN genre_revenue gr ON gt.GenreId = gr.GenreId;

-- 23. INSERT
INSERT INTO Artist (Name) VALUES ('New Artist');

-- 24. INSERT with columns
INSERT INTO Album (Title, ArtistId) VALUES ('New Album', 1);

-- 25. INSERT with subquery
INSERT INTO PlaylistTrack (PlaylistId, TrackId)
SELECT 1, TrackId FROM Track WHERE GenreId = 1;

-- 26. UPDATE
UPDATE Track SET UnitPrice = 1.29 WHERE TrackId = 1;

-- 27. UPDATE with subquery
UPDATE Customer SET SupportRepId = 3
WHERE CustomerId IN (
    SELECT i.CustomerId FROM Invoice i WHERE i.Total > 20
);

-- 28. DELETE
DELETE FROM InvoiceLine WHERE InvoiceLineId = 1;

-- 29. DELETE with subquery
DELETE FROM PlaylistTrack
WHERE TrackId IN (
    SELECT t.TrackId FROM Track t WHERE t.Bytes IS NULL
);

-- 30. DISTINCT
SELECT DISTINCT Country FROM Customer;

-- 31. ORDER BY with alias
SELECT ar.Name, COUNT(al.AlbumId) AS NumAlbums
FROM Artist ar
INNER JOIN Album al ON ar.ArtistId = al.ArtistId
GROUP BY ar.ArtistId, ar.Name
ORDER BY NumAlbums DESC;

-- 32. UNION: all people names
SELECT c.FirstName, c.LastName, 'Customer' AS Type FROM Customer c
UNION
SELECT e.FirstName, e.LastName, 'Employee' AS Type FROM Employee e;

-- 33. NULL handling
SELECT TrackId, Name, Composer FROM Track WHERE Composer IS NOT NULL;

-- 34. BETWEEN
SELECT InvoiceId, Total FROM Invoice WHERE Total BETWEEN 5.00 AND 15.00;

-- 35. LIKE pattern
SELECT FirstName, LastName FROM Customer WHERE LastName LIKE 'M%';

-- 36. Full invoice detail chain (5 tables)
SELECT i.InvoiceId, i.InvoiceDate, c.FirstName, c.LastName,
       t.Name AS TrackName, al.Title AS AlbumTitle,
       il.Quantity, il.UnitPrice
FROM Invoice i
INNER JOIN Customer c ON i.CustomerId = c.CustomerId
INNER JOIN InvoiceLine il ON i.InvoiceId = il.InvoiceId
INNER JOIN Track t ON il.TrackId = t.TrackId
INNER JOIN Album al ON t.AlbumId = al.AlbumId;

-- 37. Cross join
SELECT g.Name AS Genre, mt.Name AS MediaType
FROM Genre g
CROSS JOIN MediaType mt;

-- 38. Nested subquery in SELECT
SELECT ar.Name,
    (SELECT COUNT(DISTINCT g.GenreId)
     FROM Album al
     INNER JOIN Track t ON al.AlbumId = t.AlbumId
     INNER JOIN Genre g ON t.GenreId = g.GenreId
     WHERE al.ArtistId = ar.ArtistId) AS GenreCount
FROM Artist ar;

-- 39. Complex CTE: employee sales performance
WITH employee_sales AS (
    SELECT e.EmployeeId, e.FirstName, e.LastName,
           SUM(i.Total) AS TotalSales
    FROM Employee e
    INNER JOIN Customer c ON e.EmployeeId = c.SupportRepId
    INNER JOIN Invoice i ON c.CustomerId = i.CustomerId
    GROUP BY e.EmployeeId, e.FirstName, e.LastName
)
SELECT FirstName, LastName, TotalSales
FROM employee_sales;

-- 40. Derived table join
SELECT ar.Name, album_stats.AlbumCount, album_stats.TotalTracks
FROM Artist ar
INNER JOIN (
    SELECT al.ArtistId, COUNT(DISTINCT al.AlbumId) AS AlbumCount,
           COUNT(t.TrackId) AS TotalTracks
    FROM Album al
    LEFT JOIN Track t ON al.AlbumId = t.AlbumId
    GROUP BY al.ArtistId
) AS album_stats ON ar.ArtistId = album_stats.ArtistId;
