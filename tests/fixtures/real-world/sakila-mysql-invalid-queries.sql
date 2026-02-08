-- Invalid queries against the Sakila MySQL schema (error detection tests)

-- E0001: Table not found
SELECT * FROM movies;

-- E0002: Column not found (typo)
SELECT first_name, lst_name FROM actor;

-- E0002: Column not found
SELECT titl FROM film;

-- E0001: Table not found in JOIN
SELECT f.title, d.name
FROM film f
INNER JOIN director d ON f.director_id = d.director_id;

-- E0002: Column not found in WHERE
SELECT film_id, title FROM film WHERE genre = 'Action';

-- E0002: Column not found in JOIN condition
SELECT f.title, a.first_name
FROM film f
INNER JOIN film_actor fa ON f.film_id = fa.film_id
INNER JOIN actor a ON fa.actor_id = a.id;

-- E0002: Column not found in aggregate
SELECT rating, AVG(duration) AS avg_len FROM film GROUP BY rating;

-- E0002: Column not found (table has 'amount', not 'total')
SELECT customer_id, SUM(total) FROM payment GROUP BY customer_id;

-- E0002: Column not found in subquery
SELECT first_name, last_name FROM customer
WHERE customer_id IN (SELECT cust_id FROM rental);

-- E0005: Column count mismatch in INSERT
INSERT INTO actor (first_name, last_name)
VALUES ('JOHN', 'DOE', '2024-01-01');

-- E0002: Column not found in INSERT
INSERT INTO film (title, description, language_id, genre)
VALUES ('Test Film', 'Description', 1, 'Action');

-- E0001: Table not found in INSERT
INSERT INTO directors (first_name, last_name) VALUES ('Steven', 'Spielberg');
