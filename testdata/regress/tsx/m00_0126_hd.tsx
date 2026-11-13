   return <div>
        {...(<Todo key={todos[0].id} todo={todos[0].todo} /> as any)}
    </div>;
}
declare let x: TodoListProps;
    <TodoList {...x}/>
