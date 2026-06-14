-- SQL サンプル: lint / format を sqlfluff で検査する（postgres dialect）。
SELECT
    u.id,
    u.name,
    count(o.id) AS order_count
FROM users AS u
LEFT JOIN orders AS o
    ON u.id = o.user_id
WHERE u.active IS TRUE
GROUP BY u.id, u.name
HAVING count(o.id) > 0
ORDER BY order_count DESC;
