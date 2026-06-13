def fix_db():
    with open('server/src/postgres_db.rs', 'r', encoding='utf-8') as f:
        lines = f.readlines()
        
    new_lines = []
    in_structs = False
    
    # We want to keep lines 1-10
    # Delete 12-45
    # Keep 46-483 (connect and init_schema)
    # Then close the impl
    
    for i, line in enumerate(lines):
        line_num = i + 1
        
        if 1 <= line_num <= 10:
            new_lines.append(line)
        elif 12 <= line_num <= 45:
            continue
        elif 46 <= line_num <= 483:
            new_lines.append(line)
            
    # Add closing brace
    new_lines.append("}\n")
    
    with open('server/src/postgres_db.rs', 'w', encoding='utf-8') as f:
        f.writelines(new_lines)

if __name__ == "__main__":
    fix_db()
