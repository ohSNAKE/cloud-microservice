-- 将分类 icon 从 emoji 迁移为扁平 icon key
-- 请在备份数据库后自行决定是否执行

UPDATE categories SET icon = 'dining' WHERE icon = '🍜';
UPDATE categories SET icon = 'transport' WHERE icon = '🚗';
UPDATE categories SET icon = 'shopping' WHERE icon = '🛒';
UPDATE categories SET icon = 'housing' WHERE icon = '🏠';
UPDATE categories SET icon = 'entertainment' WHERE icon = '🎮';
UPDATE categories SET icon = 'medical' WHERE icon = '💊';
UPDATE categories SET icon = 'education' WHERE icon = '📚';
UPDATE categories SET icon = 'pushpin' WHERE icon = '📌';
UPDATE categories SET icon = 'salary' WHERE icon = '💰';
UPDATE categories SET icon = 'gift' WHERE icon = '🎁';
UPDATE categories SET icon = 'investment' WHERE icon = '📈';
UPDATE categories SET icon = 'cash-income' WHERE icon = '💵';
