import React, { useState } from 'react';
import { useTheme } from './hooks/useTheme';
import {
  Button,
  ButtonGroup,
  Input,
  TextArea,
  Checkbox,
  Radio,
  Select,
  Dropdown,
  DropdownItem,
  DropdownHeader,
  DropdownDivider,
  Alert,
  Badge,
  Table,
  TableHead,
  TableBody,
  TableRow,
  TableHeaderCell,
  TableCell,
  TablePagination,
  Card,
  CardHeader,
  CardContent,
  CardFooter,
} from './components';

const App: React.FC = () => {
  const { theme, toggleTheme } = useTheme();
  const [currentPage, setCurrentPage] = useState(1);

  return (
    <div id="app">
      {/* Titlebar */}
      <div className="titlebar">
        <div className="titlebar-title">QueryBox</div>
        <div className="titlebar-controls">
          <Button variant="ghost" size="sm" onClick={toggleTheme}>
            <span>{theme === 'light' ? '🌙' : '☀️'}</span>
          </Button>
        </div>
      </div>

      {/* Main Content */}
      <div className="main-content">
        {/* Sidebar */}
        <div className="sidebar">
          <div className="p-4">
            <h3 className="h5 mb-4">Design System Demo</h3>
            <nav>
              <a href="#buttons" className="nav-link">Buttons</a>
              <a href="#inputs" className="nav-link">Inputs</a>
              <a href="#selects" className="nav-link">Selects & Dropdowns</a>
              <a href="#tables" className="nav-link">Tables</a>
              <a href="#components" className="nav-link">Other Components</a>
            </nav>
          </div>
        </div>

        {/* Content Area */}
        <div className="content">
          {/* Buttons Section */}
          <section id="buttons" className="mb-12">
            <Card>
              <CardHeader title="Buttons" description="Various button styles and variants" />
              <CardContent>
                <div className="mb-6">
                  <h4 className="h5 mb-3">Primary Buttons</h4>
                  <div className="flex gap-2 flex-wrap">
                    <Button variant="primary" size="sm">Small</Button>
                    <Button variant="primary">Default</Button>
                    <Button variant="primary" size="lg">Large</Button>
                    <Button variant="primary" disabled>Disabled</Button>
                  </div>
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Button Variants</h4>
                  <div className="flex gap-2 flex-wrap">
                    <Button variant="primary">Primary</Button>
                    <Button variant="secondary">Secondary</Button>
                    <Button variant="outline">Outline</Button>
                    <Button variant="ghost">Ghost</Button>
                    <Button variant="success">Success</Button>
                    <Button variant="danger">Danger</Button>
                  </div>
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Button Group</h4>
                  <ButtonGroup>
                    <Button variant="secondary">Left</Button>
                    <Button variant="secondary">Middle</Button>
                    <Button variant="secondary">Right</Button>
                  </ButtonGroup>
                </div>
              </CardContent>
            </Card>
          </section>

          {/* Inputs Section */}
          <section id="inputs" className="mb-12">
            <Card>
              <CardHeader title="Input Fields" description="Text inputs, textareas, checkboxes, and radio buttons" />
              <CardContent>
                <div className="mb-6">
                  <h4 className="h5 mb-3">Text Inputs</h4>
                  <div className="flex flex-col gap-4">
                    <Input
                      label="Username"
                      placeholder="Enter username"
                      required
                    />
                    <Input
                      label="Email"
                      type="email"
                      placeholder="Enter email"
                      hint="We'll never share your email"
                    />
                    <TextArea
                      label="Description"
                      placeholder="Enter description"
                    />
                  </div>
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Checkboxes</h4>
                  <div className="flex flex-col gap-2">
                    <Checkbox label="Accept terms and conditions" defaultChecked />
                    <Checkbox label="Subscribe to newsletter" />
                    <Checkbox label="Disabled option" disabled />
                  </div>
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Radio Buttons</h4>
                  <div className="flex flex-col gap-2">
                    <Radio name="option" label="Option 1" defaultChecked />
                    <Radio name="option" label="Option 2" />
                    <Radio name="option" label="Disabled option" disabled />
                  </div>
                </div>
              </CardContent>
            </Card>
          </section>

          {/* Selects & Dropdowns Section */}
          <section id="selects" className="mb-12">
            <Card>
              <CardHeader title="Selects & Dropdowns" description="Select boxes and dropdown menus" />
              <CardContent>
                <div className="mb-6">
                  <h4 className="h5 mb-3">Select Box</h4>
                  <Select
                    options={[
                      { value: '', label: 'Choose an option' },
                      { value: '1', label: 'Option 1' },
                      { value: '2', label: 'Option 2' },
                      { value: '3', label: 'Option 3' },
                    ]}
                  />
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Dropdown Menu</h4>
                  <Dropdown trigger={<Button variant="secondary">Open Menu</Button>}>
                    <DropdownHeader>Actions</DropdownHeader>
                    <DropdownItem>Edit</DropdownItem>
                    <DropdownItem>Duplicate</DropdownItem>
                    <DropdownDivider />
                    <DropdownItem active>View</DropdownItem>
                    <DropdownDivider />
                    <DropdownItem>Delete</DropdownItem>
                  </Dropdown>
                </div>
              </CardContent>
            </Card>
          </section>

          {/* Tables Section */}
          <section id="tables" className="mb-12">
            <Card>
              <CardHeader title="Data Tables" description="Displaying tabular data" />
              <CardContent>
                <Table striped>
                  <TableHead>
                    <TableRow>
                      <TableHeaderCell className="table-checkbox">
                        <input type="checkbox" />
                      </TableHeaderCell>
                      <TableHeaderCell sortable>Name</TableHeaderCell>
                      <TableHeaderCell sortable>Email</TableHeaderCell>
                      <TableHeaderCell sortable>Role</TableHeaderCell>
                      <TableHeaderCell sortable>Status</TableHeaderCell>
                      <TableHeaderCell>Actions</TableHeaderCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    <TableRow>
                      <TableCell className="table-checkbox">
                        <input type="checkbox" />
                      </TableCell>
                      <TableCell>John Doe</TableCell>
                      <TableCell>john.doe@example.com</TableCell>
                      <TableCell>Admin</TableCell>
                      <TableCell>
                        <Badge variant="success">Active</Badge>
                      </TableCell>
                      <TableCell>
                        <div className="table-actions">
                          <button className="table-action-btn">✏️</button>
                          <button className="table-action-btn">🗑️</button>
                        </div>
                      </TableCell>
                    </TableRow>
                    <TableRow>
                      <TableCell className="table-checkbox">
                        <input type="checkbox" />
                      </TableCell>
                      <TableCell>Jane Smith</TableCell>
                      <TableCell>jane.smith@example.com</TableCell>
                      <TableCell>User</TableCell>
                      <TableCell>
                        <Badge variant="warning">Pending</Badge>
                      </TableCell>
                      <TableCell>
                        <div className="table-actions">
                          <button className="table-action-btn">✏️</button>
                          <button className="table-action-btn">🗑️</button>
                        </div>
                      </TableCell>
                    </TableRow>
                    <TableRow>
                      <TableCell className="table-checkbox">
                        <input type="checkbox" />
                      </TableCell>
                      <TableCell>Bob Johnson</TableCell>
                      <TableCell>bob.johnson@example.com</TableCell>
                      <TableCell>User</TableCell>
                      <TableCell>
                        <Badge variant="error">Inactive</Badge>
                      </TableCell>
                      <TableCell>
                        <div className="table-actions">
                          <button className="table-action-btn">✏️</button>
                          <button className="table-action-btn">🗑️</button>
                        </div>
                      </TableCell>
                    </TableRow>
                  </TableBody>
                </Table>
                <TablePagination
                  currentPage={currentPage}
                  totalPages={1}
                  onPageChange={setCurrentPage}
                  totalItems={3}
                  itemsPerPage={10}
                />
              </CardContent>
            </Card>
          </section>

          {/* Other Components Section */}
          <section id="components" className="mb-12">
            <Card>
              <CardHeader title="Other Components" description="Alerts, badges, and more" />
              <CardContent>
                <div className="mb-6">
                  <h4 className="h5 mb-3">Alerts</h4>
                  <div className="flex flex-col gap-3">
                    <Alert variant="info" title="Info" description="This is an informational message" />
                    <Alert variant="success" title="Success" description="Operation completed successfully" />
                    <Alert variant="warning" title="Warning" description="Please review before proceeding" />
                    <Alert variant="error" title="Error" description="Something went wrong" />
                  </div>
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Badges</h4>
                  <div className="flex gap-2 flex-wrap">
                    <Badge variant="primary">Primary</Badge>
                    <Badge variant="secondary">Secondary</Badge>
                    <Badge variant="success">Success</Badge>
                    <Badge variant="warning">Warning</Badge>
                    <Badge variant="error">Error</Badge>
                  </div>
                </div>

                <div className="mb-6">
                  <h4 className="h5 mb-3">Typography</h4>
                  <h1 className="h1">Heading 1</h1>
                  <h2 className="h2">Heading 2</h2>
                  <h3 className="h3">Heading 3</h3>
                  <h4 className="h4">Heading 4</h4>
                  <h5 className="h5">Heading 5</h5>
                  <h6 className="h6">Heading 6</h6>
                  <p>This is a regular paragraph with some text content. Lorem ipsum dolor sit amet, consectetur adipiscing elit.</p>
                </div>
              </CardContent>
            </Card>
          </section>
        </div>
      </div>
    </div>
  );
};

export default App;
