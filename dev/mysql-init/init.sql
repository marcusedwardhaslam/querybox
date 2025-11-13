CREATE DATABASE IF NOT EXISTS querybox;
CREATE USER IF NOT EXISTS 'queryuser'@'%' IDENTIFIED BY 'querypass';
GRANT ALL PRIVILEGES ON querybox.* TO 'queryuser'@'%';
FLUSH PRIVILEGES;

CREATE DATABASE IF NOT EXISTS querybox;
USE querybox;

CREATE TABLE users (
  id INT AUTO_INCREMENT PRIMARY KEY,
  username VARCHAR(50) NOT NULL UNIQUE,
  email VARCHAR(100) NOT NULL UNIQUE,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE orders (
  id INT AUTO_INCREMENT PRIMARY KEY,
  user_id INT NOT NULL,
  product VARCHAR(100) NOT NULL,
  amount DECIMAL(10,2) NOT NULL,
  order_date TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  FOREIGN KEY (user_id) REFERENCES users(id)
);

INSERT INTO users (username, email) VALUES
  ('alice', 'alice@example.com'),
  ('bob', 'bob@example.com'),
  ('carol', 'carol@example.com');

INSERT INTO orders (user_id, product, amount) VALUES
  (1, 'Laptop', 1200.00),
  (2, 'Phone', 800.00),
  (1, 'Mouse', 25.99),
  (3, 'Monitor', 300.00);
