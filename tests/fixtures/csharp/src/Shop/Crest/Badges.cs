namespace Shop.Crest;

public class ProjectAttrAttribute : Attribute
{
    public ProjectAttrAttribute() { }
    public ProjectAttrAttribute(string page) { Page = page; }
    public string Name { get; set; } = "";
    public string Page { get; } = "";
}

public class LocalAttribute : Attribute
{
    public bool Strict { get; set; }
}
