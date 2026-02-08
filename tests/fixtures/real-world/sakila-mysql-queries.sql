-- Valid queries against the Sakila MySQL schema
-- Tests: SELECT, JOIN, subquery, CTE, aggregate, INSERT, UPDATE, DELETE

-- 1. Basic SELECT
SELECT actor_id, first_name, last_name FROM actor;

-- 2. SELECT with WHERE
SELECT film_id, title, description, rental_rate
FROM film
WHERE rental_rate > 2.99;

-- 3. INNER JOIN two tables
SELECT f.title, l.name AS language_name
FROM film f
INNER JOIN language l ON f.language_id = l.language_id;

-- 4. Multi-table JOIN (film -> actor through junction)
SELECT a.first_name, a.last_name, f.title
FROM actor a
INNER JOIN film_actor fa ON a.actor_id = fa.actor_id
INNER JOIN film f ON fa.film_id = f.film_id;

-- 5. LEFT JOIN (films without actors)
SELECT f.title, fa.actor_id
FROM film f
LEFT JOIN film_actor fa ON f.film_id = fa.film_id;

-- 6. Three-way JOIN with category
SELECT f.title, c.name AS category_name
FROM film f
INNER JOIN film_category fc ON f.film_id = fc.film_id
INNER JOIN category c ON fc.category_id = c.category_id;

-- 7. Customer with address chain (4 tables)
SELECT cu.first_name, cu.last_name, cu.email,
       a.address, ci.city, co.country
FROM customer cu
INNER JOIN address a ON cu.address_id = a.address_id
INNER JOIN city ci ON a.city_id = ci.city_id
INNER JOIN country co ON ci.country_id = co.country_id;

-- 8. Aggregate: films per category
SELECT c.name AS category, COUNT(fc.film_id) AS film_count
FROM category c
INNER JOIN film_category fc ON c.category_id = fc.category_id
GROUP BY c.name;

-- 9. Aggregate: total revenue per store
SELECT s.store_id, SUM(p.amount) AS total_revenue
FROM store s
INNER JOIN staff st ON s.store_id = st.store_id
INNER JOIN payment p ON st.staff_id = p.staff_id
GROUP BY s.store_id;

-- 10. Aggregate: average rental duration per rating
SELECT rating, AVG(rental_duration) AS avg_duration
FROM film
GROUP BY rating;

-- 11. Aggregate with HAVING
SELECT a.first_name, a.last_name, COUNT(fa.film_id) AS film_count
FROM actor a
INNER JOIN film_actor fa ON a.actor_id = fa.actor_id
GROUP BY a.actor_id, a.first_name, a.last_name
HAVING COUNT(fa.film_id) > 20;

-- 12. Subquery in WHERE (IN)
SELECT first_name, last_name
FROM customer
WHERE address_id IN (
    SELECT a.address_id FROM address a WHERE a.city_id IN (
        SELECT ci.city_id FROM city ci WHERE ci.country_id = 1
    )
);

-- 13. Subquery in WHERE (EXISTS)
SELECT f.title
FROM film f
WHERE EXISTS (
    SELECT 1 FROM inventory i WHERE i.film_id = f.film_id
);

-- 14. Correlated subquery
SELECT c.first_name, c.last_name,
    (SELECT COUNT(*) FROM rental r WHERE r.customer_id = c.customer_id) AS rental_count
FROM customer c;

-- 15. Subquery in FROM (derived table)
SELECT sub.category, sub.film_count
FROM (
    SELECT c.name AS category, COUNT(fc.film_id) AS film_count
    FROM category c
    INNER JOIN film_category fc ON c.category_id = fc.category_id
    GROUP BY c.name
) AS sub
WHERE sub.film_count > 50;

-- 16. CTE: top customers by payment
WITH customer_totals AS (
    SELECT c.customer_id, c.first_name, c.last_name,
           SUM(p.amount) AS total_spent
    FROM customer c
    INNER JOIN payment p ON c.customer_id = p.customer_id
    GROUP BY c.customer_id, c.first_name, c.last_name
)
SELECT first_name, last_name, total_spent
FROM customer_totals
WHERE total_spent > 100;

-- 17. CTE: film inventory per store
WITH film_inventory AS (
    SELECT f.title, s.store_id, COUNT(i.inventory_id) AS copies
    FROM film f
    INNER JOIN inventory i ON f.film_id = i.film_id
    INNER JOIN store s ON i.store_id = s.store_id
    GROUP BY f.title, s.store_id
)
SELECT title, store_id, copies
FROM film_inventory
WHERE copies > 3;

-- 18. Multiple CTEs
WITH active_customers AS (
    SELECT customer_id, first_name, last_name
    FROM customer
    WHERE active = TRUE
),
customer_rentals AS (
    SELECT ac.customer_id, ac.first_name, ac.last_name,
           COUNT(r.rental_id) AS rental_count
    FROM active_customers ac
    INNER JOIN rental r ON ac.customer_id = r.customer_id
    GROUP BY ac.customer_id, ac.first_name, ac.last_name
)
SELECT first_name, last_name, rental_count
FROM customer_rentals;

-- 19. INSERT into actor
INSERT INTO actor (first_name, last_name)
VALUES ('JOHN', 'DOE');

-- 20. INSERT into rental
INSERT INTO rental (rental_date, inventory_id, customer_id, staff_id)
VALUES ('2024-01-01 10:00:00', 1, 1, 1);

-- 21. INSERT with subquery
INSERT INTO film_text (film_id, title, description)
SELECT film_id, title, description FROM film WHERE film_id = 1;

-- 22. UPDATE single table
UPDATE film SET rental_rate = 3.99
WHERE film_id = 1;

-- 23. UPDATE with subquery in WHERE
UPDATE customer SET active = FALSE
WHERE customer_id IN (
    SELECT r.customer_id FROM rental r
    WHERE r.return_date IS NULL
);

-- 24. DELETE
DELETE FROM payment WHERE payment_id = 1;

-- 25. DELETE with subquery
DELETE FROM film_actor
WHERE film_id IN (
    SELECT f.film_id FROM film f WHERE f.rating = 'NC-17'
);

-- 26. Self-join on address/city hierarchy
SELECT ci.city, co.country
FROM city ci
INNER JOIN country co ON ci.country_id = co.country_id;

-- 27. ENUM column in WHERE
SELECT film_id, title, rating
FROM film
WHERE rating = 'PG-13';

-- 28. ORDER BY with alias
SELECT c.name AS category, COUNT(fc.film_id) AS total_films
FROM category c
INNER JOIN film_category fc ON c.category_id = fc.category_id
GROUP BY c.name
ORDER BY total_films DESC;

-- 29. NULL handling
SELECT film_id, title, original_language_id
FROM film
WHERE original_language_id IS NULL;

-- 30. BETWEEN and comparison
SELECT title, rental_rate, replacement_cost
FROM film
WHERE rental_rate BETWEEN 2.99 AND 4.99
  AND replacement_cost < 20.00;

-- 31. LIKE pattern
SELECT first_name, last_name
FROM actor
WHERE last_name LIKE 'S%';

-- 32. DISTINCT
SELECT DISTINCT rating FROM film;

-- 33. UNION
SELECT a.first_name, a.last_name, 'actor' AS person_type FROM actor a
UNION
SELECT c.first_name, c.last_name, 'customer' AS person_type FROM customer c;

-- 34. Rental + payment combined analysis
SELECT r.rental_id, r.rental_date, p.amount, p.payment_date,
       c.first_name, c.last_name
FROM rental r
INNER JOIN payment p ON r.rental_id = p.rental_id
INNER JOIN customer c ON r.customer_id = c.customer_id;

-- 35. Staff and store with address
SELECT st.first_name, st.last_name, st.email,
       s.store_id, a.address, ci.city
FROM staff st
INNER JOIN store s ON st.store_id = s.store_id
INNER JOIN address a ON st.address_id = a.address_id
INNER JOIN city ci ON a.city_id = ci.city_id;

-- 36. Complex aggregate: most popular films
WITH rental_counts AS (
    SELECT i.film_id, COUNT(r.rental_id) AS times_rented
    FROM inventory i
    INNER JOIN rental r ON i.inventory_id = r.inventory_id
    GROUP BY i.film_id
)
SELECT f.title, rc.times_rented
FROM film f
INNER JOIN rental_counts rc ON f.film_id = rc.film_id
ORDER BY rc.times_rented DESC;

-- 37. Cross join (all film-language combinations)
SELECT f.title, l.name AS lang
FROM film f
CROSS JOIN language l;

-- 38. Inventory availability check
SELECT f.title, s.store_id,
       COUNT(i.inventory_id) AS in_stock
FROM film f
INNER JOIN inventory i ON f.film_id = i.film_id
INNER JOIN store s ON i.store_id = s.store_id
LEFT JOIN rental r ON i.inventory_id = r.inventory_id AND r.return_date IS NULL
WHERE r.rental_id IS NULL
GROUP BY f.title, s.store_id;

-- 39. Multiple aggregates
SELECT c.name AS category,
       COUNT(f.film_id) AS num_films,
       AVG(f.rental_rate) AS avg_rate,
       SUM(f.replacement_cost) AS total_cost,
       MIN(f.length) AS shortest,
       MAX(f.length) AS longest
FROM category c
INNER JOIN film_category fc ON c.category_id = fc.category_id
INNER JOIN film f ON fc.film_id = f.film_id
GROUP BY c.name;

-- 40. Nested derived table
SELECT top_cats.category, top_cats.avg_rate
FROM (
    SELECT sub.category, sub.avg_rate
    FROM (
        SELECT c.name AS category, AVG(f.rental_rate) AS avg_rate
        FROM category c
        INNER JOIN film_category fc ON c.category_id = fc.category_id
        INNER JOIN film f ON fc.film_id = f.film_id
        GROUP BY c.name
    ) AS sub
    WHERE sub.avg_rate > 3.00
) AS top_cats;
